#!/bin/sh
# nextest run wrapper for the `qualify` profile: each test process gets a new database of its own
# on the server RESTLESS_TEST_ADMIN_URL names, dropped when it exits. SQLx serialises migrations
# with one lock per database, so a shared database ran every test's company-schema migrations one
# at a time (most of a 10-minute run). A database each also keeps tests from racing on schemas
# they share, such as restless_authority.
set -u
admin="${RESTLESS_TEST_ADMIN_URL:?set RESTLESS_TEST_ADMIN_URL to a database on a disposable server}"
database="t$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')_test"
psql "$admin" -qAtc "CREATE DATABASE $database" >/dev/null || exit 1
RESTLESS_TEST_DATABASE_URL="${admin%/*}/$database" "$@"
status=$?
psql "$admin" -qAtc "DROP DATABASE $database WITH (FORCE)" >/dev/null 2>&1
exit "$status"
