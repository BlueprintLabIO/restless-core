# UXG-133 — Provide a way to create and edit company know-how

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | high | missing-function | US-080, US-081 |

## Problem

The skills model says owner surfaces only "select, accept, retire and assign"; there is no create or edit route or UI. Know-how is the main way an owner shapes how the company works and it cannot be authored or corrected here.

## Evidence

- `web/src/lib/model/skills.ts:227-232`
- `crates/restless-owner/src/owner.rs:1330-1337`

## Suggested direction

"New know-how" opens a native document that becomes a candidate skill; "Edit" opens its SKILL.md.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
