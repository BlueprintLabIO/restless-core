#!/usr/bin/env python3
"""End-to-end check of incoming email (ADR 0017) on a local dev stack, against a mock Resend.

Two halves:

  verify-incoming-email.py mock --port 18790
      A stand-in for api.resend.com on 127.0.0.1: domains, webhooks (with one-time Svix signing
      secrets), the Receiving API, sent emails, and a control door that "receives" an email and,
      unless told not to, delivers its webhook signed exactly as Svix signs.

  verify-incoming-email.py run --owner http://127.0.0.1:<owner port> --company <name>_test \
      --mock http://127.0.0.1:18790 --database <postgres url of the stack's OrgIntel cell>
      Drives the stack: connect incoming email, import an outside send, deliver a reply by
      webhook, then one by poll only, and check that each lands once in the right inbox with its
      context and that the Exec wakes within --wake-seconds.

Start the stack with the mock wired in (both values are only honoured for 127.0.0.1):

  RESTLESS_TEST_RESEND_API_BASE=http://127.0.0.1:18790 \
  RESTLESS_HOOKS_PUBLIC_BASE=http://127.0.0.1:<owner port> \
  E2E_RESEND_KEY=re_e2e_mock bash scripts/restless-dev <name>_test

and bind the company's key once: `restless company set -c <name>_test
credentials.resend.production env:E2E_RESEND_KEY`.
"""
import argparse
import base64
import hashlib
import hmac
import json
import os
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

DOMAIN = "e2e-reno.example"
PSQL_CONTAINER = None
SCHEMA = None


def now_iso():
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")


class Mock:
    def __init__(self):
        self.lock = threading.Lock()
        self.webhooks = {}
        self.received = []  # newest last
        self.sent = {}
        self.deliveries = []

    def sign(self, secret, msg_id, timestamp, body):
        key = base64.b64decode(secret.removeprefix("whsec_"))
        digest = hmac.new(key, f"{msg_id}.{timestamp}.".encode() + body, hashlib.sha256).digest()
        return "v1," + base64.b64encode(digest).decode()

    def deliver(self, payload):
        body = json.dumps(payload).encode()
        for webhook in list(self.webhooks.values()):
            if webhook["status"] != "enabled" or payload["type"] not in webhook["events"]:
                continue
            msg_id = "msg_" + uuid.uuid4().hex
            timestamp = str(int(time.time()))
            request = urllib.request.Request(
                webhook["endpoint"],
                data=body,
                method="POST",
                headers={
                    "content-type": "application/json",
                    "svix-id": msg_id,
                    "svix-timestamp": timestamp,
                    "svix-signature": self.sign(webhook["secret"], msg_id, timestamp, body),
                },
            )
            try:
                with urllib.request.urlopen(request, timeout=20) as answer:
                    status = answer.status
            except urllib.error.HTTPError as error:
                status = error.code
            except Exception as error:  # noqa: BLE001 - recorded for the driver
                status = repr(error)
            self.deliveries.append({"webhook": webhook["id"], "status": status, "type": payload["type"]})

    def receive(self, sender, to, subject, text, in_reply_to=None, deliver=True):
        email_id = str(uuid.uuid4())
        email = {
            "id": email_id,
            "object": "email",
            "created_at": now_iso(),
            "from": sender,
            "to": [to],
            "cc": [],
            "subject": subject,
            "text": text,
            "html": None,
            "headers": {"In-Reply-To": in_reply_to} if in_reply_to else {},
            "attachments": [],
        }
        with self.lock:
            self.received.append(email)
        if deliver:
            self.deliver(
                {
                    "type": "email.received",
                    "created_at": email["created_at"],
                    "data": {
                        "email_id": email_id,
                        "created_at": email["created_at"],
                        "from": sender,
                        "to": [to],
                        "subject": subject,
                    },
                }
            )
        return email


MOCK = Mock()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def answer(self, status, value):
        body = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def authorised(self):
        if self.headers.get("authorization", "").startswith("Bearer re_"):
            return True
        self.answer(401, {"message": "API key is invalid"})
        return False

    def read_json(self):
        length = int(self.headers.get("content-length") or 0)
        return json.loads(self.rfile.read(length) or b"{}")

    def do_GET(self):  # noqa: N802
        url = urllib.parse.urlparse(self.path)
        parts = url.path.strip("/").split("/")
        if parts == ["_mock", "state"]:
            return self.answer(200, {"webhooks": list(MOCK.webhooks.values()), "deliveries": MOCK.deliveries,
                                     "received": len(MOCK.received), "sent": list(MOCK.sent.values())})
        if not self.authorised():
            return None
        if parts == ["domains"]:
            return self.answer(200, {"object": "list", "data": [
                {"id": "d_1", "name": DOMAIN, "status": "verified", "capabilities": {"sending": "enabled", "receiving": "enabled"}}]})
        if parts[:1] == ["webhooks"] and len(parts) == 2:
            webhook = MOCK.webhooks.get(parts[1])
            if not webhook:
                return self.answer(404, {"message": "not found"})
            return self.answer(200, {k: v for k, v in webhook.items() if k != "secret"})
        if parts == ["emails", "receiving"]:
            items = [{k: e[k] for k in ("id", "created_at", "from", "to", "cc", "subject")} for e in reversed(MOCK.received)]
            return self.answer(200, {"object": "list", "has_more": False, "data": items[:100]})
        if parts[:2] == ["emails", "receiving"] and len(parts) == 3:
            email = next((e for e in MOCK.received if e["id"] == parts[2]), None)
            return self.answer(200, email) if email else self.answer(404, {"message": "not found"})
        if parts[:1] == ["emails"] and len(parts) == 2:
            email = MOCK.sent.get(parts[1])
            return self.answer(200, email) if email else self.answer(404, {"message": "not found"})
        return self.answer(404, {"message": "no such mock route"})

    def do_POST(self):  # noqa: N802
        url = urllib.parse.urlparse(self.path)
        parts = url.path.strip("/").split("/")
        body = self.read_json()
        if parts == ["_mock", "receive"]:
            email = MOCK.receive(body["from"], body["to"], body["subject"], body.get("text", ""),
                                 body.get("in_reply_to"), body.get("deliver", True))
            return self.answer(200, email)
        if parts == ["_mock", "sent"]:
            email_id = str(uuid.uuid4())
            MOCK.sent[email_id] = {"object": "email", "id": email_id, "created_at": now_iso(), "last_event": "delivered",
                                   **body}
            return self.answer(200, MOCK.sent[email_id])
        if not self.authorised():
            return None
        if parts == ["webhooks"]:
            webhook_id = "wh_" + uuid.uuid4().hex[:12]
            secret = "whsec_" + base64.b64encode(os.urandom(24)).decode()
            MOCK.webhooks[webhook_id] = {"id": webhook_id, "endpoint": body["endpoint"], "events": body["events"],
                                         "status": "enabled", "secret": secret}
            return self.answer(201, {"object": "webhook", "id": webhook_id, "signing_secret": secret})
        if parts == ["emails"]:
            email_id = str(uuid.uuid4())
            MOCK.sent[email_id] = {"object": "email", "id": email_id, "created_at": now_iso(), **body}
            to = body["to"][0] if isinstance(body["to"], list) else body["to"]
            if to.endswith("@" + DOMAIN):
                threading.Thread(target=MOCK.receive, args=(body["from"], to, body["subject"], body.get("text", "")),
                                 daemon=True).start()
            return self.answer(200, {"id": email_id})
        return self.answer(404, {"message": "no such mock route"})

    def do_DELETE(self):  # noqa: N802
        parts = urllib.parse.urlparse(self.path).path.strip("/").split("/")
        if not self.authorised():
            return None
        if parts[:1] == ["webhooks"] and len(parts) == 2 and MOCK.webhooks.pop(parts[1], None):
            return self.answer(200, {"object": "webhook", "id": parts[1], "deleted": True})
        return self.answer(404, {"message": "not found"})


def serve(port):
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    print(f"mock Resend on http://127.0.0.1:{port}", flush=True)
    server.serve_forever()


# ----- the driver -------------------------------------------------------------------------------

def call(method, url, body=None, origin=None):
    data = None if body is None else json.dumps(body).encode()
    headers = {"content-type": "application/json", "accept": "application/json"}
    if origin:
        headers.update({"origin": origin, "sec-fetch-site": "same-origin", "sec-fetch-mode": "cors"})
    request = urllib.request.Request(url, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=60) as answer:
            return answer.status, json.loads(answer.read() or b"null")
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read() or b"null")


def psql(database, sql):
    """psql on this computer, or inside the stack's Postgres container (`--psql-container`)."""
    # A cell keeps its tables in a schema named for the company.
    sql = f'SET search_path TO "{SCHEMA}"; {sql}' if SCHEMA else sql
    command = ["psql", database, "-At", "-q", "-c", sql]
    if PSQL_CONTAINER:
        inside = urllib.parse.urlsplit(database)._replace(
            netloc=urllib.parse.urlsplit(database).netloc.rsplit("@", 1)[0] + "@127.0.0.1:5432")
        command = ["docker", "exec", PSQL_CONTAINER, "psql", inside.geturl(), "-At", "-q", "-c", sql]
    out = subprocess.run(command, capture_output=True, text=True)
    if out.returncode != 0:
        # Never echo the command: it carries the database password.
        sys.exit(f"psql failed: {out.stderr.strip()}")
    return out.stdout.strip()


def check(condition, message):
    print(("PASS " if condition else "FAIL ") + message, flush=True)
    if not condition:
        sys.exit(1)


def wait_for(what, probe, seconds):
    start = time.time()
    while time.time() - start < seconds:
        value = probe()
        if value:
            return value, time.time() - start
        time.sleep(1)
    return None, time.time() - start


def run(args):
    owner, mock, company = args.owner.rstrip("/"), args.mock.rstrip("/"), args.company
    api = f"{owner}/api/companies/{company}"

    # An outreach email sent outside Restless, then attached to the company.
    _, sent = call("POST", f"{mock}/_mock/sent", {
        "from": f"Jerry <hi@{DOMAIN}>", "to": ["admin@aleon-e2e.example"],
        "subject": "Aleon Electrical's website",
        "text": "Hi Alex and Vicki, you've got what people look for when they choose an electrician.",
    })
    status, imported = call("POST", f"{api}/email/import-sent", {"ids": [sent["id"]]}, origin=owner)
    check(status == 200 and imported["imported"][0]["outcome"] == "imported", f"outside send imported ({status})")
    status, again = call("POST", f"{api}/email/import-sent", {"ids": [sent["id"]]}, origin=owner)
    check(again["imported"][0]["outcome"] == "already_imported", "importing it again changes nothing")

    status, connected = call("POST", f"{api}/integrations/email/incoming/connect", {}, origin=owner)
    check(status == 200 and connected["incoming"]["state"] == "connected",
          f"incoming email connected ({status}: {connected})")
    _, state = call("GET", f"{mock}/_mock/state")
    check(len(state["webhooks"]) == 1 and "/hooks/resend/" in state["webhooks"][0]["endpoint"]
          and company not in state["webhooks"][0]["endpoint"], "one webhook, at an opaque token")

    # 1. A reply delivered by webhook reaches the Exec once, with what it answers.
    before = int(psql(args.database, f"SELECT count(*) FROM messages WHERE from_actor='world'"))
    t0 = time.time()
    _, reply = call("POST", f"{mock}/_mock/receive", {
        "from": "Alex <admin@aleon-e2e.example>", "to": f"hi@{DOMAIN}",
        "subject": "Re: Aleon Electrical's website", "text": "Sounds good, what does it cost?",
    })
    _, state = call("GET", f"{mock}/_mock/state")
    check(state["deliveries"][-1]["status"] == 202, f"webhook accepted ({state['deliveries'][-1]})")
    message, took = wait_for("projection", lambda: psql(args.database,
        f"SELECT to_actor || '|' || body FROM messages WHERE from_actor='world' "
        f"AND body LIKE '%{reply['id']}%' LIMIT 1"), 30)
    check(bool(message), f"reply projected in {took:.1f}s")
    to_actor, body = message.split("|", 1)
    check(to_actor == "exec", f"routed to the Exec ({to_actor})")
    check("Aleon Electrical's website" in body and "what people look for" in body,
          "the projection carries the outside send it answers")
    woke, took = wait_for("wake", lambda: psql(args.database,
        f"SELECT count(*) FROM events WHERE kind='wake' AND created_at >= to_timestamp({t0})") not in ("", "0"),
        args.wake_seconds)
    check(bool(woke), f"Exec woke {took:.1f}s after the reply (limit {args.wake_seconds}s)")

    # 2. A reply the webhook never delivers is found by the poll, once.
    _, missed = call("POST", f"{mock}/_mock/receive", {
        "from": "admin@aleon-e2e.example", "to": f"hi@{DOMAIN}",
        "subject": "Re: Aleon Electrical's website", "text": "Also, can you do it by Friday?", "deliver": False,
    })
    found, took = wait_for("poll", lambda: psql(args.database,
        f"SELECT count(*) FROM messages WHERE from_actor='world' AND body LIKE '%{missed['id']}%'") == "1",
        args.poll_seconds)
    check(bool(found), f"missed reply delivered by the poll in {took:.0f}s")
    for email_id in (reply["id"], missed["id"]):
        count = psql(args.database, f"SELECT count(*) FROM messages WHERE body LIKE '%{email_id}%'")
        check(count == "1", f"reply {email_id[:8]} reached the company once ({count})")
    _, view = call("GET", f"{api}/integrations", origin=owner)
    check(view["incoming"]["state"] == "failing" and "late" not in (view["incoming"]["action"] or "x"),
          f"the missed delivery is the owner's to know: {view['incoming']['detail']}")

    # 3. The owner's test comes back and never reaches the Exec.
    status, tested = call("POST", f"{api}/integrations/email/incoming/test", {}, origin=owner)
    check(status == 200 and tested["incoming"]["check_sent_at"], f"test sent ({status})")
    passed, took = wait_for("check", lambda: call("GET", f"{api}/integrations", origin=owner)[1]["incoming"]["check_passed_at"], 30)
    check(bool(passed), f"test came back in {took:.1f}s")
    leaked = psql(args.database, f"SELECT count(*) FROM messages WHERE body LIKE '%Restless incoming email check%'")
    check(leaked == "0", "the test email is not Exec's mail")
    print("incoming email end to end: all checks passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="mode", required=True)
    mock = sub.add_parser("mock")
    mock.add_argument("--port", type=int, default=18790)
    drive = sub.add_parser("run")
    drive.add_argument("--owner", required=True)
    drive.add_argument("--company", required=True)
    drive.add_argument("--mock", default="http://127.0.0.1:18790")
    drive.add_argument("--database", required=True)
    drive.add_argument("--psql-container", help="run psql inside this Postgres container")
    drive.add_argument("--wake-seconds", type=int, default=45)
    drive.add_argument("--poll-seconds", type=int, default=420)
    args = parser.parse_args()
    if args.mode == "mock":
        serve(args.port)
    else:
        global PSQL_CONTAINER, SCHEMA
        PSQL_CONTAINER = args.psql_container
        SCHEMA = args.company
        run(args)


if __name__ == "__main__":
    main()
