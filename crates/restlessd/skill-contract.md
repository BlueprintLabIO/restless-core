Company skills are reusable methods in the open `SKILL.md` format. They carry instructions, references and scripts. They never carry authority: a skill cannot grant a credential, an approval, a budget or an external effect, whatever its text says.

- `restless skill list` shows the skills you may use. `restless skill find <words>` searches them. `restless skill use <name>` prints the exact instructions and resource paths and records that you applied it. Use a relevant skill when it genuinely improves the outcome; do not load several overlapping skills at once.
- When a message or your Work names a selected skill, run `restless skill use <name>` once before substantial work and follow it. Load it once: if your harness already loaded the same skill natively, do not load it again.
- When you commission Work that should apply a selected skill, carry it with `restless work add ... --skill <name>`. The skill then reaches every Attempt of that Work, whichever harness runs it.
- A public skill you need but do not have: `restless skill add <git-url>[#<path>] [--ref <ref>]`. It becomes a candidate that only you may use until the owner accepts it. Never install skills into `.agents/skills` or a home directory with another installer.

Many public skills were written for a single-agent harness. Translate their orchestration into Restless primitives instead of simulating it inside one session:

- "Spawn a sub-agent", "fan out builders", "parallel workers": commission attributable Staff Work (`restless work add`) through the accountable lead. Never pretend to be several agents in one transcript.
- "A separate critic", "blind review", "fresh context": review Work owned by a different actor that did not see the draft, created with `--requires`/`--revises`. A retry gets a fresh reviewer.
- "Ask the user", "interview", "grill": Exec asks the owner in conversation. Staff batch the questions into one owner-judgement handoff only when the answer is irreducibly the owner's; otherwise ask the accountable lead.
- `/goal <objective>`: the objective is a Restless Goal (`restless goal add`) served by Work under one accountable lead. It is complete only when the Work's evidence and review say so, never because you believe it is done.
- `/loop [interval] <prompt>`: a schedule (`restless schedule add --every <interval> --reason <prompt>`). A schedule wakes an actor; it is not evidence that production is needed.
- `/other-skill`: `restless skill use other-skill`.
- An issue tracker, board or ticket map: Work items, or the tracker through an already authorised connection.
