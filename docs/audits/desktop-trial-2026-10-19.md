# Desktop trial: TigerVNC + noVNC against KasmVNC (19 October 2026)

The owner asked whether a faster remote desktop was worth adopting, and agreed KasmVNC would be
tried with a before/after measurement and adopted only if it is better and the observer/controller
boundary still holds (S99 §Desktop).

## Method

One container built on the company image, two displays showing the same page: a scrolling,
image-heavy listing feed (what an agent's browser looks like while it works), 1280×800.

- `:1` today's stack: Xtigervnc, websockify, noVNC 1.6 (the company desktop as shipped).
- `:2` KasmVNC 1.5.0 (`kasmvncserver_noble_1.5.0_amd64.deb`), its own websocket server and web client.

A headless Chromium opened each web client view-only for 20–25 s and counted the frames that
actually changed on screen (a hash of the canvas centre each animation frame) and the websocket
bytes received. Links were shaped inside the container with `tc netem`: unthrottled, 20 Mbit/s with
45 ms each way (about Sydney to Singapore), and 8 Mbit/s with the same delay (a typical home upload,
which is what a desktop on the owner's own PC rides on).

## Results

| Stack | Link | Frames/s | p95 frame gap | Mbit/s |
| --- | --- | ---: | ---: | ---: |
| noVNC, quality 9 (as shipped) | direct | 27.9 | 56 ms | 74.2 |
| noVNC, quality 6 | direct | 32.6 | 49 ms | 33.8 |
| KasmVNC | direct | 24.4 | 52 ms | 15.5 |
| noVNC, quality 9 (as shipped) | 20 Mbit, 90 ms | **6.0** | 200 ms | 16.4 |
| noVNC, quality 6 | 20 Mbit, 90 ms | 18.3 | 67 ms | 19.0 |
| KasmVNC | 20 Mbit, 90 ms | **24.5** | 51 ms | 15.6 |
| noVNC, quality 9 (as shipped) | 8 Mbit, 90 ms | **2.6** | 431 ms | 7.2 |
| noVNC, quality 6 | 8 Mbit, 90 ms | 7.0 | 194 ms | 7.5 |
| noVNC, quality 4 | 8 Mbit, 90 ms | 7.5 | 174 ms | 6.5 |
| KasmVNC | 8 Mbit, 90 ms | **11.8** | 133 ms | 7.7 |

## Decision

1. **Now:** the cockpit's noVNC client asks for JPEG quality 6 and compression 6 instead of 9 and
   2. About three times the frame rate on real links, no protocol change, nothing else touched.
   Quality 4 adds little over 6.
2. **KasmVNC is better and worth adopting**: it holds about 24 fps where today's stack gets 6, and
   1.7× quality 6 on a home uplink, using its own adaptive encoder. It is not a drop-in swap. The
   plane enforces view-only by parsing the RFB client stream (`owner_desktop.rs`
   `RfbObserverFilter`, which expects a standard RFB 3.8 handshake and message set), and the
   cockpit embeds noVNC's `RFB` directly. Kasm's web client speaks RFB with its own extensions.
   Adoption is a sprint of its own: teach the filter Kasm's client messages (or enforce view-only
   in KasmVNC's own per-connection permissions), embed Kasm's client, keep the display-lease and
   resize contract, and repeat this measurement plus the observer-input refusal test.
3. No GPU per cloud computer and no Sunshine/Moonlight, as agreed: neither is needed for these
   frame rates, and both need native clients or GPUs.

Script and page: kept with the trial under `~/code/work/kasm-trial` on the core machine
(`measure.mjs`, `motion.html`, `start.sh`, `Dockerfile`).
