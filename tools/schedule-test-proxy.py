#!/usr/bin/env python3
"""Narrow egress and host-port sidecar for isolated schedule actor runs.

Run this only inside a dual-homed Docker sidecar attached to the bridge and
the actor's internal test network. The actor should join only that internal
network; the proxy does not create or enforce Docker network topology.

Required environment:
  RESTLESS_TEST_RELAY_LISTEN_PORT    local sidecar relay port (e.g. 8790)
  RESTLESS_TEST_RELAY_HOST_PORT       exact host model-relay TCP port
  RESTLESS_TEST_COORDINATOR_LISTEN_PORT local sidecar coordinator port (e.g. 8791)
  RESTLESS_TEST_COORDINATOR_HOST_PORT exact host coordinator TCP port

The sidecar exposes a CONNECT-only model proxy on :8080 and TCP forwarders on
the exact required local ports. The forwarders connect only to
host.docker.internal and the corresponding required host port.
"""

from __future__ import annotations

import os
import re
import socket
import socketserver
import threading
import time


MODEL_HOSTS = frozenset({"api.openai.com", "chatgpt.com", "auth.openai.com"})
MODEL_PROXY_PORT = 8080
HOST_GATEWAY = "host.docker.internal"
HEADER_LIMIT = 8192
HEADER_TIMEOUT = 5.0
CONNECT_TIMEOUT = 8.0
TUNNEL_IDLE_TIMEOUT = 900.0
COPY_CHUNK = 64 * 1024


def required_port(name: str) -> int:
    value = os.environ.get(name)
    if value is None or not re.fullmatch(r"[1-9][0-9]{0,4}", value):
        raise SystemExit(f"{name} must be an explicitly supplied decimal TCP port")
    port = int(value)
    if port > 65535:
        raise SystemExit(f"{name} must be a TCP port in 1..65535")
    return port


def reject(client: socket.socket, status: bytes = b"403 Forbidden") -> None:
    try:
        client.sendall(b"HTTP/1.1 " + status + b"\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
    except OSError:
        pass


def read_headers(client: socket.socket) -> tuple[bytes, bytes] | None:
    """Read one bounded header block, preserving bytes following its terminator."""
    data = bytearray()
    client.settimeout(HEADER_TIMEOUT)
    while len(data) < HEADER_LIMIT:
        chunk = client.recv(min(1024, HEADER_LIMIT - len(data)))
        if not chunk:
            return None
        data.extend(chunk)
        end = data.find(b"\r\n\r\n")
        if end >= 0:
            return bytes(data[:end]), bytes(data[end + 4 :])
    return None


def tunnel(left: socket.socket, right: socket.socket, initial: bytes = b"") -> None:
    """Pump both directions with blocking sendall, preserving TCP backpressure."""
    left.settimeout(TUNNEL_IDLE_TIMEOUT)
    right.settimeout(TUNNEL_IDLE_TIMEOUT)
    if initial:
        right.sendall(initial)

    def pump(source: socket.socket, destination: socket.socket) -> None:
        try:
            while True:
                data = source.recv(COPY_CHUNK)
                if not data:
                    try:
                        destination.shutdown(socket.SHUT_WR)
                    except OSError:
                        pass
                    return
                destination.sendall(data)
        except OSError:
            # A timeout or transport error ends both directions; EOF above
            # only half-closes the peer so its final response can still drain.
            for peer in (left, right):
                try:
                    peer.shutdown(socket.SHUT_RDWR)
                except OSError:
                    pass

    pumps = [
        threading.Thread(target=pump, args=(left, right), daemon=True),
        threading.Thread(target=pump, args=(right, left), daemon=True),
    ]
    for worker in pumps:
        worker.start()
    for worker in pumps:
        worker.join()


class ThreadingServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True
    block_on_close = False


class ModelProxyHandler(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        client = self.request
        upstream: socket.socket | None = None
        try:
            parsed = read_headers(client)
            if parsed is None:
                reject(client, b"400 Bad Request")
                return
            raw_headers, early_data = parsed
            lines = raw_headers.split(b"\r\n")
            request = lines[0].split(b" ")
            # No absolute-form HTTP requests or other proxy methods are accepted.
            if len(request) != 3 or request[0] != b"CONNECT" or request[2] not in (b"HTTP/1.0", b"HTTP/1.1"):
                reject(client)
                return
            try:
                authority = request[1].decode("ascii")
            except UnicodeDecodeError:
                reject(client)
                return
            if authority.count(":") != 1:
                reject(client)
                return
            host, port = authority.rsplit(":", 1)
            host = host.lower()
            if host not in MODEL_HOSTS or port != "443":
                reject(client)
                return
            upstream = socket.create_connection((host, 443), timeout=CONNECT_TIMEOUT)
            upstream.settimeout(None)
            client.settimeout(None)
            client.sendall(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            tunnel(client, upstream, early_data)
        except (OSError, TimeoutError):
            # Deliberately omit exception strings and request contents from logs.
            reject(client, b"502 Bad Gateway")
        finally:
            if upstream is not None:
                upstream.close()


class ForwardHandler(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        target_port = self.server.target_port  # type: ignore[attr-defined]
        upstream: socket.socket | None = None
        try:
            upstream = socket.create_connection((HOST_GATEWAY, target_port), timeout=CONNECT_TIMEOUT)
            tunnel(self.request, upstream)
        except (OSError, TimeoutError):
            pass
        finally:
            if upstream is not None:
                upstream.close()


def forward_server(listen_port: int, target_port: int) -> ThreadingServer:
    server = ThreadingServer(("0.0.0.0", listen_port), ForwardHandler)
    server.target_port = target_port  # type: ignore[attr-defined]
    return server


def main() -> None:
    relay_listen_port = required_port("RESTLESS_TEST_RELAY_LISTEN_PORT")
    relay_host_port = required_port("RESTLESS_TEST_RELAY_HOST_PORT")
    coordinator_listen_port = required_port("RESTLESS_TEST_COORDINATOR_LISTEN_PORT")
    coordinator_host_port = required_port("RESTLESS_TEST_COORDINATOR_HOST_PORT")
    if relay_listen_port == coordinator_listen_port:
        raise SystemExit("relay and coordinator listen ports must differ")
    if MODEL_PROXY_PORT in (relay_listen_port, coordinator_listen_port):
        raise SystemExit("forwarder listen ports must differ from the model proxy port 8080")
    servers = [
        ThreadingServer(("0.0.0.0", MODEL_PROXY_PORT), ModelProxyHandler),
        forward_server(relay_listen_port, relay_host_port),
        forward_server(coordinator_listen_port, coordinator_host_port),
    ]
    threads = []
    for server in servers:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        threads.append(thread)
    print("schedule test sidecar listeners ready", flush=True)
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        pass
    finally:
        for server in servers:
            server.shutdown()
            server.server_close()
        for thread in threads:
            thread.join(timeout=1)


if __name__ == "__main__":
    main()
