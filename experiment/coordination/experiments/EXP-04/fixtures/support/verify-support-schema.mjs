#!/usr/bin/env node
import fs from "node:fs";
const files = process.argv.slice(2).filter(x => x.endsWith(".json"));
const expectedArg = process.argv.slice(2).find(x => x.startsWith("ids="));
const units = files.flatMap(file => JSON.parse(fs.readFileSync(file, "utf8")));
const expected = expectedArg ? expectedArg.slice(4).split(",").filter(Boolean) : units.map(u=>u.id);
const required = ["id","policy_version","action","customer_safe_draft","system_action_plan","next_state"];
const errors = [];
if (new Set(units.map(u=>u.id)).size !== units.length) errors.push("duplicates");
if (units.map(u=>u.id).sort().join() !== [...expected].sort().join()) errors.push("ownership");
for (const unit of units) for (const key of required) if (unit[key] === undefined || unit[key] === "") errors.push(`${unit.id}:${key}`);
if (errors.length) { console.error(JSON.stringify(errors)); process.exit(1); }
console.log(JSON.stringify({valid:true, units:units.length}));
