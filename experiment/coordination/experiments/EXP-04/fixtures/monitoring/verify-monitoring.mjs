#!/usr/bin/env node
import fs from "node:fs";
const docs = JSON.parse(fs.readFileSync("corpus/documents.json", "utf8"));
const entities = [...new Set(docs.map(d => d.entity))].sort();
const requestedFiles = process.argv.slice(2).filter(x => x.endsWith(".json"));
const requestedArg = process.argv.slice(2).find(x => x.startsWith("entities="));
const files = requestedFiles.length ? requestedFiles : (fs.existsSync("alerts") ? fs.readdirSync("alerts").filter(x => x.endsWith(".json")).map(x => `alerts/${x}`) : []);
const alerts = files.flatMap(file => JSON.parse(fs.readFileSync(file, "utf8")));
const expectedEntities = requestedArg ? requestedArg.slice(9).split(",").filter(Boolean) : entities;
const errors = [];
if (new Set(alerts.map(a => a.entity)).size !== alerts.length) errors.push("duplicate entities");
if (alerts.map(a=>a.entity).sort().join() !== [...expectedEntities].sort().join()) errors.push("entity ownership mismatch");
for (const alert of alerts) {
  const index = Number(alert.entity.slice(-2));
  const expected = {event_code:`EVENT-${String(index).padStart(2,"0")}`, severity:index%4===0?"high":index%2===0?"medium":"low", follow_up_trigger:`TRIGGER-${String(index).padStart(2,"0")}`};
  for (const [key,value] of Object.entries(expected)) if (alert[key] !== value) errors.push(`${alert.entity}:${key}`);
  const sources = [...(alert.source_ids || [])].sort();
  if (sources.join() !== [`D${String(index).padStart(2,"0")}-LATE`,`D${String(index).padStart(2,"0")}-OFFICIAL`].sort().join()) errors.push(`${alert.entity}:sources`);
  if (!String(alert.uncertainty || "").trim()) errors.push(`${alert.entity}:uncertainty`);
}
if (errors.length) { console.error(JSON.stringify(errors)); process.exit(1); }
console.log(JSON.stringify({valid:true, alerts:alerts.length, files:files.sort()}));
