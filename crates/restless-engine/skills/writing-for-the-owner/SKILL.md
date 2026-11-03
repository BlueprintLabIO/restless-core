---
name: writing-for-the-owner
description: How to write what the owner reads - Work titles and outcomes, resolutions, Attempt summaries, artifact and gate labels, and documents you share. Read before writing any of them in a session.
---

Some of what you write is rendered straight to the owner. Work titles and outcomes, Work
resolutions, Attempt summaries, artifact labels and gate names all appear on the owner's Work
surface exactly as you wrote them. The owner is not the actor that executes them.

Where a field is both an instruction to an actor and something the owner reads, write both, in that
order, separated by a blank line:

1. **Open with one or two plain sentences a non-technical owner can read.** Name what will exist
   when this is done and why it matters to the business. No file paths, flags, gate names, command
   names, ids, or internal vocabulary in that opening. Do not restate the title.
2. **Then the exact contract, unchanged.** Every constraint, path, gate, banned value, source of
   evidence and honesty rule stays exactly as precise as it needs to be. The readable opening is an
   addition, never a replacement and never a softening: if making it read well would drop a
   requirement, keep the requirement and rewrite the sentence.

A wall of imperative instructions with no readable opening is the current failure. So is an opening
that sells the work, invents significance the evidence does not support, or hides a constraint the
executing actor needs.

Give artifacts and gates short names that say what they are, not what command produced them or every
condition they satisfy. Use a recognisable noun phrase for the label (for example, “Homepage review”
or “Browser check report”) and put purpose or qualification in the separate note. Do not reuse the
full expected-artifact contract as a label. An owner scanning the evidence should be able to tell
what each one is without opening it.

Assume the reader does not know the repository, toolchain or team shorthand. The opening must still
make sense when read by itself. Use a concrete subject and active verb, one idea per sentence and
familiar words. Expand an acronym the first time it matters. If the owner must translate a path,
command, status token or agent role to learn what exists, the opening is not finished.

This is presentation of the same truth, not a second version of it. Never write an owner-facing
opening that claims more than the contract below it delivers.

A document you share with a person or send for review must be finished work. Start with the content
itself. Leave out Work ids, message numbers, Restless commands, company-computer paths and
placeholders such as `[LINK TO …]`: say what a thing is in words, or link it. What changed and who
changed it goes in the version note (`document checkpoint --reason`), which people see in the
document's history, never at the top of the document. Restless checks this when you share or send
for review and refuses a document that is not ready, saying what to fix.

Show a picture by embedding it: `![what it shows](/company/outputs/x.png)`, or in a document a
path relative to it. PNG, JPEG, GIF and WebP show in place; convert anything else to PNG. A
`file://` link, `localhost` or a path outside `/company` cannot be shown.

A place you name (an address, a venue, a suburb) links to Google Maps, so the reader can see it:
`[Bondi Pavilion](https://www.google.com/maps/search/?api=1&query=Bondi%20Pavilion)`. On a page you
build, follow the map pattern in the company-tools skill.
