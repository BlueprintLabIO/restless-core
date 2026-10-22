//! Byte streams multiplexed over one websocket between a plane and a machine.
//!
//! Each websocket binary message is one frame: a type byte, a big-endian stream id, a payload.
//! Either end may open a stream; the plane's ids are even and the machine's odd, so they never
//! collide. Every stream has its own credit window, so a slow reader stops only its own sender and
//! never the link: memory per stream is bounded by [`WINDOW`].
//!
//! The link knows nothing about websockets. [`Link::new`] returns the frames to send as a channel,
//! and the owner of the socket hands every received frame to [`Link::receive`].

use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{mpsc, oneshot};

/// Bytes a sender may have in flight on one stream before its reader catches up.
pub const WINDOW: u32 = 256 * 1024;
/// Largest data frame, well inside a websocket message limit.
const MAX_CHUNK: usize = 32 * 1024;
const MAX_OPEN_PAYLOAD: usize = 1024;
const OPEN_TIMEOUT: Duration = Duration::from_secs(30);

const OPEN: u8 = 1;
const ACCEPT: u8 = 2;
const REFUSE: u8 = 3;
const DATA: u8 = 4;
const CREDIT: u8 = 5;
const FIN: u8 = 6;
const RESET: u8 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Plane,
    Machine,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenPayload {
    kind: String,
    target: String,
}

enum Chunk {
    Data(Bytes),
    Fin,
    Reset,
}

#[derive(Default)]
struct Credit {
    available: u64,
    waker: Option<Waker>,
    closed: bool,
}

struct Slot {
    data: mpsc::UnboundedSender<Chunk>,
    credit: Arc<Mutex<Credit>>,
    opened: Option<oneshot::Sender<Result<(), String>>>,
}

struct Shared {
    slots: Mutex<HashMap<u32, Slot>>,
    out: mpsc::UnboundedSender<Vec<u8>>,
    next_id: AtomicU32,
    closed: AtomicBool,
}

impl Shared {
    fn send(&self, kind: u8, id: u32, payload: &[u8]) -> bool {
        let mut frame = Vec::with_capacity(5 + payload.len());
        frame.push(kind);
        frame.extend_from_slice(&id.to_be_bytes());
        frame.extend_from_slice(payload);
        self.out.send(frame).is_ok()
    }

    fn new_slot(
        &self,
        id: u32,
        opened: Option<oneshot::Sender<Result<(), String>>>,
    ) -> Option<(mpsc::UnboundedReceiver<Chunk>, Arc<Mutex<Credit>>)> {
        let (data, rx) = mpsc::unbounded_channel();
        let credit = Arc::new(Mutex::new(Credit {
            available: u64::from(WINDOW),
            ..Credit::default()
        }));
        let mut slots = self.slots.lock().unwrap();
        if self.closed.load(Ordering::SeqCst) || slots.contains_key(&id) {
            return None;
        }
        slots.insert(
            id,
            Slot {
                data,
                credit: Arc::clone(&credit),
                opened,
            },
        );
        Some((rx, credit))
    }
}

/// One end of a link. Cheap to clone; every clone is the same link.
#[derive(Clone)]
pub struct Link {
    shared: Arc<Shared>,
}

/// A stream the other end asked to open. Accept it or refuse it; dropping it refuses.
pub struct Incoming {
    pub kind: String,
    pub target: String,
    stream: Option<LinkStream>,
}

impl Incoming {
    pub fn accept(mut self) -> LinkStream {
        let stream = self
            .stream
            .take()
            .expect("an incoming stream is answered once");
        stream.shared.send(ACCEPT, stream.id, &[]);
        stream
    }

    pub fn refuse(mut self, reason: &str) {
        if let Some(stream) = self.stream.take() {
            stream.refuse(reason);
        }
    }
}

impl Drop for Incoming {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            stream.refuse("not accepted");
        }
    }
}

impl Link {
    /// A new link, and the frames it wants sent, in order, as websocket binary messages.
    pub fn new(side: Side) -> (Self, mpsc::UnboundedReceiver<Vec<u8>>) {
        let (out, frames) = mpsc::unbounded_channel();
        let first = match side {
            Side::Plane => 2,
            Side::Machine => 1,
        };
        let shared = Arc::new(Shared {
            slots: Mutex::new(HashMap::new()),
            out,
            next_id: AtomicU32::new(first),
            closed: AtomicBool::new(false),
        });
        (Self { shared }, frames)
    }

    pub fn is_closed(&self) -> bool {
        self.shared.closed.load(Ordering::SeqCst)
    }

    /// Open a stream of `kind` to `target`; the other end decides whether to accept it.
    pub async fn open(&self, kind: &str, target: &str) -> io::Result<LinkStream> {
        let payload = serde_json::to_vec(&OpenPayload {
            kind: kind.into(),
            target: target.into(),
        })
        .map_err(io::Error::other)?;
        if payload.len() > MAX_OPEN_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "stream target too long",
            ));
        }
        let id = self.shared.next_id.fetch_add(2, Ordering::SeqCst);
        let (opened, answer) = oneshot::channel();
        let (rx, credit) = self
            .shared
            .new_slot(id, Some(opened))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "link closed"))?;
        if !self.shared.send(OPEN, id, &payload) {
            self.shared.slots.lock().unwrap().remove(&id);
            return Err(io::Error::new(io::ErrorKind::NotConnected, "link closed"));
        }
        let stream = LinkStream::new(id, Arc::clone(&self.shared), rx, credit);
        match tokio::time::timeout(OPEN_TIMEOUT, answer).await {
            Ok(Ok(Ok(()))) => Ok(stream),
            Ok(Ok(Err(reason))) => {
                stream.forget();
                Err(io::Error::new(io::ErrorKind::PermissionDenied, reason))
            }
            Ok(Err(_)) => {
                stream.forget();
                Err(io::Error::new(io::ErrorKind::NotConnected, "link closed"))
            }
            Err(_) => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "the other end did not answer",
            )),
        }
    }

    /// Hand the link one frame received from the websocket. A stream the other end opened comes
    /// back to be accepted or refused.
    pub fn receive(&self, frame: &[u8]) -> Option<Incoming> {
        if frame.len() < 5 {
            return None;
        }
        let kind = frame[0];
        let id = u32::from_be_bytes([frame[1], frame[2], frame[3], frame[4]]);
        let payload = &frame[5..];
        if kind == OPEN {
            let Ok(open) = serde_json::from_slice::<OpenPayload>(payload) else {
                self.shared.send(REFUSE, id, b"malformed open");
                return None;
            };
            let Some((rx, credit)) = self.shared.new_slot(id, None) else {
                self.shared.send(RESET, id, &[]);
                return None;
            };
            let stream = LinkStream::new(id, Arc::clone(&self.shared), rx, credit);
            return Some(Incoming {
                kind: open.kind,
                target: open.target,
                stream: Some(stream),
            });
        }
        let mut slots = self.shared.slots.lock().unwrap();
        match kind {
            ACCEPT => {
                if let Some(opened) = slots.get_mut(&id).and_then(|slot| slot.opened.take()) {
                    let _ = opened.send(Ok(()));
                }
            }
            REFUSE => {
                if let Some(mut slot) = slots.remove(&id) {
                    if let Some(opened) = slot.opened.take() {
                        let _ = opened.send(Err(String::from_utf8_lossy(payload).into_owned()));
                    }
                }
            }
            DATA => {
                if let Some(slot) = slots.get(&id) {
                    let _ = slot.data.send(Chunk::Data(Bytes::copy_from_slice(payload)));
                }
            }
            CREDIT if payload.len() == 4 => {
                if let Some(slot) = slots.get(&id) {
                    let grant =
                        u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    let mut credit = slot.credit.lock().unwrap();
                    credit.available = credit.available.saturating_add(u64::from(grant));
                    if let Some(waker) = credit.waker.take() {
                        waker.wake();
                    }
                }
            }
            FIN => {
                if let Some(slot) = slots.get(&id) {
                    let _ = slot.data.send(Chunk::Fin);
                }
            }
            RESET => {
                if let Some(slot) = slots.remove(&id) {
                    reset_slot(slot, "reset by the other end");
                }
            }
            _ => {}
        }
        None
    }

    /// The websocket ended: every stream fails and no new one opens.
    pub fn close(&self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        let slots: Vec<Slot> = self
            .shared
            .slots
            .lock()
            .unwrap()
            .drain()
            .map(|(_, slot)| slot)
            .collect();
        for slot in slots {
            reset_slot(slot, "link closed");
        }
    }
}

fn reset_slot(mut slot: Slot, reason: &str) {
    let _ = slot.data.send(Chunk::Reset);
    if let Some(opened) = slot.opened.take() {
        let _ = opened.send(Err(reason.into()));
    }
    let mut credit = slot.credit.lock().unwrap();
    credit.closed = true;
    if let Some(waker) = credit.waker.take() {
        waker.wake();
    }
}

/// One stream of a link: bytes in order, both ways, with half-close.
pub struct LinkStream {
    id: u32,
    shared: Arc<Shared>,
    rx: mpsc::UnboundedReceiver<Chunk>,
    pending: Bytes,
    unacknowledged: u32,
    credit: Arc<Mutex<Credit>>,
    read_done: bool,
    write_done: bool,
    forgotten: bool,
}

impl LinkStream {
    fn new(
        id: u32,
        shared: Arc<Shared>,
        rx: mpsc::UnboundedReceiver<Chunk>,
        credit: Arc<Mutex<Credit>>,
    ) -> Self {
        Self {
            id,
            shared,
            rx,
            pending: Bytes::new(),
            unacknowledged: 0,
            credit,
            read_done: false,
            write_done: false,
            forgotten: false,
        }
    }

    fn refuse(self, reason: &str) {
        let reason: String = reason.chars().take(200).collect();
        self.shared.send(REFUSE, self.id, reason.as_bytes());
        self.forget();
    }

    /// Drop without telling the other end, which already knows the stream is gone.
    fn forget(mut self) {
        self.forgotten = true;
        self.shared.slots.lock().unwrap().remove(&self.id);
    }
}

impl AsyncRead for LinkStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = &mut *self;
        while this.pending.is_empty() {
            if this.read_done {
                return Poll::Ready(Ok(()));
            }
            match this.rx.poll_recv(cx) {
                Poll::Ready(Some(Chunk::Data(bytes))) => this.pending = bytes,
                Poll::Ready(Some(Chunk::Fin)) => this.read_done = true,
                Poll::Ready(Some(Chunk::Reset)) | Poll::Ready(None) => {
                    this.read_done = true;
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::ConnectionReset,
                        "stream reset",
                    )));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        let n = this.pending.len().min(buf.remaining());
        buf.put_slice(&this.pending.split_to(n));
        this.unacknowledged += n as u32;
        if this.unacknowledged >= WINDOW / 2 {
            this.shared
                .send(CREDIT, this.id, &this.unacknowledged.to_be_bytes());
            this.unacknowledged = 0;
        }
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for LinkStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.write_done {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stream closed for writing",
            )));
        }
        if data.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let n = {
            let mut credit = self.credit.lock().unwrap();
            if credit.closed {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "stream reset",
                )));
            }
            if credit.available == 0 {
                credit.waker = Some(cx.waker().clone());
                return Poll::Pending;
            }
            let n = data.len().min(MAX_CHUNK).min(credit.available as usize);
            credit.available -= n as u64;
            n
        };
        if !self.shared.send(DATA, self.id, &data[..n]) {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "link closed",
            )));
        }
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        if !self.write_done {
            self.write_done = true;
            self.shared.send(FIN, self.id, &[]);
        }
        Poll::Ready(Ok(()))
    }
}

impl Drop for LinkStream {
    fn drop(&mut self) {
        if self.forgotten {
            return;
        }
        let still_known = self.shared.slots.lock().unwrap().remove(&self.id).is_some();
        if still_known && !(self.read_done && self.write_done) {
            self.shared.send(RESET, self.id, &[]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Two ends joined directly, as if by a websocket. Streams the machine end is asked to open
    /// arrive on the returned channel.
    fn joined() -> (
        Link,
        Link,
        mpsc::UnboundedReceiver<Incoming>,
        mpsc::UnboundedReceiver<Incoming>,
    ) {
        let (plane, mut to_machine) = Link::new(Side::Plane);
        let (machine, mut to_plane) = Link::new(Side::Machine);
        let (machine_incoming, machine_rx) = mpsc::unbounded_channel();
        let (plane_incoming, plane_rx) = mpsc::unbounded_channel();
        let machine_end = machine.clone();
        tokio::spawn(async move {
            while let Some(frame) = to_machine.recv().await {
                if let Some(incoming) = machine_end.receive(&frame) {
                    let _ = machine_incoming.send(incoming);
                }
            }
            machine_end.close();
        });
        let plane_end = plane.clone();
        tokio::spawn(async move {
            while let Some(frame) = to_plane.recv().await {
                if let Some(incoming) = plane_end.receive(&frame) {
                    let _ = plane_incoming.send(incoming);
                }
            }
            plane_end.close();
        });
        (plane, machine, plane_rx, machine_rx)
    }

    #[tokio::test]
    async fn a_stream_carries_more_than_its_window_both_ways() {
        let (plane, _machine, _plane_rx, mut machine_rx) = joined();
        tokio::spawn(async move {
            let incoming = machine_rx.recv().await.unwrap();
            assert_eq!(
                (incoming.kind.as_str(), incoming.target.as_str()),
                ("docker", "x")
            );
            let mut stream = incoming.accept();
            let mut received = Vec::new();
            stream.read_to_end(&mut received).await.unwrap();
            stream.write_all(&received).await.unwrap();
            stream.shutdown().await.unwrap();
        });
        let mut stream = plane.open("docker", "x").await.unwrap();
        let sent: Vec<u8> = (0..(3 * WINDOW as usize + 17))
            .map(|i| (i % 251) as u8)
            .collect();
        let writer = {
            let sent = sent.clone();
            async move {
                stream.write_all(&sent).await.unwrap();
                stream.shutdown().await.unwrap();
                let mut echoed = Vec::new();
                stream.read_to_end(&mut echoed).await.unwrap();
                echoed
            }
        };
        let echoed = tokio::time::timeout(Duration::from_secs(10), writer)
            .await
            .unwrap();
        assert_eq!(echoed, sent);
    }

    #[tokio::test]
    async fn a_refused_stream_says_why() {
        let (plane, _machine, _plane_rx, mut machine_rx) = joined();
        tokio::spawn(async move {
            machine_rx
                .recv()
                .await
                .unwrap()
                .refuse("not a Restless container");
        });
        let error = plane.open("connect", "postgres:5432").await.err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "not a Restless container");
    }

    #[tokio::test]
    async fn the_machine_end_opens_streams_too() {
        let (_plane, machine, mut plane_rx, _machine_rx) = joined();
        tokio::spawn(async move {
            let mut stream = plane_rx.recv().await.unwrap().accept();
            stream.write_all(b"pong").await.unwrap();
            stream.shutdown().await.unwrap();
        });
        let mut stream = machine.open("plane", "7300").await.unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).await.unwrap();
        assert_eq!(answer, "pong");
    }

    #[tokio::test]
    async fn a_closed_link_fails_its_streams() {
        let (plane, machine, _plane_rx, mut machine_rx) = joined();
        let accepted = tokio::spawn(async move { machine_rx.recv().await.unwrap().accept() });
        let mut stream = plane.open("docker", "x").await.unwrap();
        let _held = accepted.await.unwrap();
        plane.close();
        let mut buffer = [0u8; 4];
        assert!(stream.read(&mut buffer).await.is_err());
        assert!(plane.open("docker", "x").await.is_err());
        drop(machine);
    }

    #[tokio::test]
    async fn dropping_a_stream_resets_the_other_end() {
        let (plane, _machine, _plane_rx, mut machine_rx) = joined();
        let reader = tokio::spawn(async move {
            let mut stream = machine_rx.recv().await.unwrap().accept();
            let mut buffer = Vec::new();
            stream.read_to_end(&mut buffer).await
        });
        let stream = plane.open("docker", "x").await.unwrap();
        drop(stream);
        assert!(reader.await.unwrap().is_err());
    }
}
