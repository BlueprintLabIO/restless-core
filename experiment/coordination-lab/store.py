from __future__ import annotations

import json
import sqlite3
import time
import uuid
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Iterator


SCHEMA = """
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS actors (
  id TEXT PRIMARY KEY,
  role TEXT NOT NULL,
  brief TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS work (
  id TEXT PRIMARY KEY,
  created_by TEXT NOT NULL REFERENCES actors(id),
  owner TEXT NOT NULL REFERENCES actors(id),
  outcome TEXT NOT NULL,
  expected_artifact TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('active','blocked','completed','abandoned')),
  revision INTEGER NOT NULL DEFAULT 1,
  base_ref TEXT NOT NULL,
  branch TEXT NOT NULL,
  worktree TEXT NOT NULL,
  feedback TEXT,
  created_at REAL NOT NULL,
  updated_at REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS work_edges (
  work_id TEXT NOT NULL REFERENCES work(id),
  other_work_id TEXT NOT NULL REFERENCES work(id),
  kind TEXT NOT NULL CHECK(kind IN ('requires','revises')),
  PRIMARY KEY(work_id, other_work_id, kind)
);
CREATE TABLE IF NOT EXISTS gates (
  work_id TEXT NOT NULL REFERENCES work(id),
  position INTEGER NOT NULL,
  name TEXT NOT NULL,
  argv_json TEXT NOT NULL,
  PRIMARY KEY(work_id, position)
);
CREATE TABLE IF NOT EXISTS attempts (
  id TEXT PRIMARY KEY,
  work_id TEXT NOT NULL REFERENCES work(id),
  revision INTEGER NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('running','produced','blocked','abandoned','failed','unknown')),
  summary TEXT,
  started_at REAL NOT NULL,
  ended_at REAL,
  UNIQUE(work_id, revision, state) ON CONFLICT IGNORE
);
CREATE UNIQUE INDEX IF NOT EXISTS one_running_attempt_per_work
  ON attempts(work_id) WHERE state='running';
CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  work_id TEXT NOT NULL REFERENCES work(id),
  attempt_id TEXT NOT NULL REFERENCES attempts(id),
  kind TEXT NOT NULL,
  reference TEXT NOT NULL,
  observed INTEGER NOT NULL DEFAULT 0,
  created_at REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  sender TEXT NOT NULL REFERENCES actors(id),
  recipient TEXT NOT NULL REFERENCES actors(id),
  body TEXT NOT NULL,
  refs_json TEXT NOT NULL,
  read_at REAL,
  created_at REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS judgements (
  id TEXT PRIMARY KEY,
  requested_by TEXT NOT NULL REFERENCES actors(id),
  assigned_to TEXT NOT NULL,
  subject TEXT NOT NULL,
  question TEXT NOT NULL,
  resume_condition TEXT NOT NULL,
  state TEXT NOT NULL CHECK(state IN ('open','resolved','declined')),
  choice TEXT,
  rationale TEXT,
  created_at REAL NOT NULL,
  resolved_at REAL
);
CREATE TABLE IF NOT EXISTS decisions (
  id TEXT PRIMARY KEY,
  decided_by TEXT NOT NULL REFERENCES actors(id),
  subject TEXT NOT NULL,
  choice TEXT NOT NULL,
  rationale TEXT NOT NULL,
  evidence_json TEXT NOT NULL,
  created_at REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS schedules (
  id TEXT PRIMARY KEY,
  actor TEXT NOT NULL REFERENCES actors(id),
  reason TEXT NOT NULL,
  not_before REAL NOT NULL,
  fired_at REAL
);
CREATE TABLE IF NOT EXISTS commands (
  id TEXT PRIMARY KEY,
  actor TEXT NOT NULL,
  name TEXT NOT NULL,
  args_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  created_at REAL NOT NULL
);
CREATE TABLE IF NOT EXISTS outbox (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  target TEXT NOT NULL,
  cause TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at REAL NOT NULL,
  delivered_at REAL
);
CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  at REAL NOT NULL,
  actor TEXT,
  kind TEXT NOT NULL,
  payload_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS turns (
  id TEXT PRIMARY KEY,
  actor TEXT NOT NULL,
  attempt_id TEXT,
  started_at REAL NOT NULL,
  ended_at REAL,
  cost_usd REAL,
  used_tokens INTEGER,
  output_tokens INTEGER,
  tool_calls INTEGER,
  end_kind TEXT,
  transcript TEXT
);
"""


def connect(path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(path, timeout=30, isolation_level=None)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys=ON")
    conn.execute("PRAGMA busy_timeout=30000")
    return conn


def initialize(path: str) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    conn = connect(path)
    conn.executescript(SCHEMA)
    conn.close()


@contextmanager
def transaction(conn: sqlite3.Connection) -> Iterator[sqlite3.Connection]:
    conn.execute("BEGIN IMMEDIATE")
    try:
        yield conn
    except Exception:
        conn.execute("ROLLBACK")
        raise
    else:
        conn.execute("COMMIT")


def uid(prefix: str) -> str:
    return f"{prefix}-{uuid.uuid4().hex[:10]}"


def emit(conn: sqlite3.Connection, kind: str, payload: dict[str, Any], actor: str | None = None) -> int:
    cur = conn.execute(
        "INSERT INTO events(at,actor,kind,payload_json) VALUES(?,?,?,?)",
        (time.time(), actor, kind, json.dumps(payload, sort_keys=True)),
    )
    return int(cur.lastrowid)


def wake(conn: sqlite3.Connection, target: str, cause: str, payload: dict[str, Any]) -> int:
    cur = conn.execute(
        "INSERT INTO outbox(target,cause,payload_json,created_at) VALUES(?,?,?,?)",
        (target, cause, json.dumps(payload, sort_keys=True), time.time()),
    )
    emit(conn, "wake_requested", {"target": target, "cause": cause, **payload})
    return int(cur.lastrowid)


def json_rows(rows: list[sqlite3.Row]) -> list[dict[str, Any]]:
    return [dict(row) for row in rows]


def snapshot(conn: sqlite3.Connection, actor: str) -> dict[str, Any]:
    work = json_rows(conn.execute("SELECT * FROM work ORDER BY created_at").fetchall())
    attempts = json_rows(conn.execute("SELECT * FROM attempts ORDER BY started_at").fetchall())
    edges = json_rows(conn.execute("SELECT * FROM work_edges ORDER BY work_id,kind").fetchall())
    artifacts = json_rows(conn.execute("SELECT * FROM artifacts ORDER BY created_at").fetchall())
    messages = json_rows(
        conn.execute(
            "SELECT * FROM messages WHERE recipient=? ORDER BY created_at", (actor,)
        ).fetchall()
    )
    judgements = json_rows(
        conn.execute(
            "SELECT * FROM judgements WHERE state='open' AND assigned_to IN (?, 'owner') ORDER BY created_at",
            (actor,),
        ).fetchall()
    )
    decisions = json_rows(conn.execute("SELECT * FROM decisions ORDER BY created_at").fetchall())
    actors = json_rows(conn.execute("SELECT id,role,brief,active FROM actors ORDER BY id").fetchall())
    for collection in (work, attempts, edges, artifacts, messages, judgements, decisions):
        for row in collection:
            for key in list(row):
                if key.endswith("_json") and row[key]:
                    row[key[:-5]] = json.loads(row.pop(key))
    return {
        "actors": actors,
        "work": work,
        "edges": edges,
        "attempts": attempts,
        "artifacts": artifacts,
        "messages_for_actor": messages,
        "open_judgements": judgements,
        "decisions": decisions,
    }
