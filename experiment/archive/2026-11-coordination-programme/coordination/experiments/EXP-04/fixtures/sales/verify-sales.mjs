#!/usr/bin/env node
import fs from "node:fs";
const accounts = JSON.parse(fs.readFileSync("data/accounts.json", "utf8"));
const byId = new Map(accounts.map(a => [a.id, a]));
const requestedFiles = process.argv.slice(2).filter(x => x.endsWith(".json"));
const requestedIdsArg = process.argv.slice(2).find(x => x.startsWith("ids="));
const files = requestedFiles.length ? requestedFiles : (fs.existsSync("outputs") ? fs.readdirSync("outputs").filter(x => x.endsWith(".json")).map(x => `outputs/${x}`) : []);
const units = files.flatMap(file => JSON.parse(fs.readFileSync(file, "utf8")));
const expectedIds = requestedIdsArg ? requestedIdsArg.slice(4).split(",").filter(Boolean) : accounts.map(a => a.id);
const ids = units.map(u => u.id);
const errors = [];
if (new Set(ids).size !== ids.length) errors.push("duplicate IDs");
if ([...ids].sort().join() !== [...expectedIds].sort().join()) errors.push(`ownership mismatch expected=${expectedIds.length} observed=${ids.length}`);
for (const unit of units) {
  const a = byId.get(unit.id); if (!a) { errors.push(`unknown ${unit.id}`); continue; }
  let q, d, t, days;
  if (a.region === "Restricted-Zone" || a.employees < 20) [q,d,t,days] = ["disqualify","closed-policy","no-contact",0];
  else if (a.fit_score >= 70 && a.intent) [q,d,t,days] = ["qualify","sales-ready","discovery",7];
  else [q,d,t,days] = ["nurture","nurture","value-resource",21];
  const exact = {qualification:q, disposition:d, action_type:t, follow_up_days:days, claim_code:a.regulated ? "evidence-only" : "standard"};
  for (const [key,value] of Object.entries(exact)) if (unit[key] !== value) errors.push(`${unit.id}:${key}`);
  if (!Array.isArray(unit.evidence) || !["employees","fit_score","intent","region"].every(k => unit.evidence.some(e => String(e).startsWith(`${k}=`)))) errors.push(`${unit.id}:evidence`);
  if (!String(unit.next_action || "").includes(a.signal)) errors.push(`${unit.id}:personalization`);
}
if (errors.length) { console.error(JSON.stringify(errors)); process.exit(1); }
console.log(JSON.stringify({valid:true, units:units.length, files:files.sort()}));
