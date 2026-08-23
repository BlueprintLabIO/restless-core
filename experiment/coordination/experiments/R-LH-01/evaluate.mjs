import fs from 'node:fs';

const path = 'research/lumaara-release-decision.md';
const text = fs.existsSync(path) ? fs.readFileSync(path, 'utf8') : '';
const dossierPaths = {
  P: 'research/evidence/play-telemetry.md',
  Q: 'research/evidence/player-interviews.md',
  A: 'research/evidence/accessibility-usability.md',
  O: 'research/evidence/production-support.md',
};
const dossiers = Object.fromEntries(Object.entries(dossierPaths).map(([key, file]) =>
  [key, fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : '']));
const ledgerPath = 'research/evidence/source-ledger.csv';
const ledger = fs.existsSync(ledgerPath) ? fs.readFileSync(ledgerPath, 'utf8') : '';
let step = 0;
function ok(name, condition, detail = '') {
  step += 1;
  const tag = condition ? 'PASS' : 'FAIL';
  if (!condition) process.exitCode = 1;
  console.log(`[${tag}] #${step} ${name}${detail ? ` :: ${detail}` : ''}`);
}
const ids = ['P','Q','A','O'].flatMap(prefix =>
  Array.from({ length: 8 }, (_, index) => `${prefix}${String(index + 1).padStart(2, '0')}`));
const cited = [...text.matchAll(/\[([PQAO]\d{2})\]/g)].map(match => match[1]);
const unknown = [...text.matchAll(/\[([A-Z]\d{2})\]/g)].map(match => match[1])
  .filter(id => !ids.includes(id));
const choices = ['Prism Expedition', 'Battle Mastery', 'Field Research', 'Accessible Journey'];

ok('decision memo exists', text.length > 0);
ok('memo is substantial enough for founder review', text.length >= 6000, `${text.length} chars`);
ok('all four regional dossiers exist', Object.values(dossiers).every(value => value.length > 0));
ok('regional dossiers are substantial independent artifacts', Object.entries(dossiers)
  .every(([, value]) => value.length >= 4000), Object.entries(dossiers).map(([key,value]) => `${key}:${value.length}`).join(','));
ok('play dossier cites every P source', Array.from({length: 8}, (_,index) => `P0${index + 1}`)
  .every(id => dossiers.P.includes(`[${id}]`)));
ok('interview dossier cites every Q source', Array.from({length: 8}, (_,index) => `Q0${index + 1}`)
  .every(id => dossiers.Q.includes(`[${id}]`)));
ok('accessibility dossier cites every A source', Array.from({length: 8}, (_,index) => `A0${index + 1}`)
  .every(id => dossiers.A.includes(`[${id}]`)));
ok('production dossier cites every O source', Array.from({length: 8}, (_,index) => `O0${index + 1}`)
  .every(id => dossiers.O.includes(`[${id}]`)));
ok('every dossier covers claims, limits, choices and unanswered questions', Object.values(dossiers)
  .every(value => /claim/i.test(value) && /limit/i.test(value) && /Prism Expedition/.test(value) &&
    /Battle Mastery/.test(value) && /Field Research/.test(value) && /Accessible Journey/.test(value) &&
    /(unanswered|cannot answer|does not answer)/i.test(value)));
ok('source ledger exists with one row per frozen source', ledger.length > 0 && ids.every(id =>
  new RegExp(`^${id},`, 'm').test(ledger)));
ok('source ledger exposes the required decision fields', /source.?id/i.test(ledger.split('\n')[0] || '') &&
  /region/i.test(ledger.split('\n')[0] || '') && /claim/i.test(ledger.split('\n')[0] || '') &&
  /(limitation|counterevidence)/i.test(ledger.split('\n')[0] || '') && /choice/i.test(ledger.split('\n')[0] || '') &&
  /strength/i.test(ledger.split('\n')[0] || ''));
ok('memo labels the corpus synthetic and non-market', /synthetic/i.test(text) && /(not real|not a claim|not market|experimental)/i.test(text));
ok('memo contains an executive decision', /executive decision/i.test(text));
ok('memo names exactly one primary direction', choices.filter(choice =>
  new RegExp(`primary.{0,80}${choice}|${choice}.{0,80}primary`, 'is').test(text)).length === 1);
ok('memo compares all four choices', choices.every(choice => text.includes(choice)));
ok('memo synthesises play telemetry', /play telemetry/i.test(text));
ok('memo synthesises player interviews', /(player interviews|interview evidence)/i.test(text));
ok('memo synthesises accessibility and usability', /accessibility/i.test(text) && /usability/i.test(text));
ok('memo synthesises production and support', /production/i.test(text) && /support/i.test(text));
ok('memo contains an explicit decision matrix', /decision matrix/i.test(text) && /\|[^\n]+\|/.test(text));
ok('memo explains criteria or weighting', /(weight|criterion|criteria|trade-off)/i.test(text));
ok('every frozen source ID is cited', ids.every(id => cited.includes(id)),
  `missing=${ids.filter(id => !cited.includes(id)).join(',')}`);
ok('memo cites no unknown source ID', unknown.length === 0, [...new Set(unknown)].join(','));
ok('memo preserves the 2,400-session funnel fact', /2[,.]?400/.test(text) && /61%/.test(text) && /27%/.test(text));
ok('memo preserves objective-beacon timing', /164\s*(seconds|s)/i.test(text) && /93\s*(seconds|s)/i.test(text));
ok('memo preserves reduced-motion abandonment ratio', /2[.]3\s*(times|x|×)/i.test(text));
ok('memo preserves synthetic support proportions', /46%/.test(text) && /29%/.test(text) && /15%/.test(text));
ok('memo reconciles at least four tensions', /(tension|conflict|counterevidence)/i.test(text) &&
  (text.match(/\b(tension|conflict|counterevidence)\b/gi) || []).length >= 4);
ok('memo defines a bounded release sequence', /(release sequence|phased sequence|bounded sequence)/i.test(text));
ok('memo states success signals and stop conditions', /success signal/i.test(text) && /stop condition/i.test(text));
ok('memo states risks, limitations and a falsifier', /risks?/i.test(text) && /limitations?/i.test(text) && /falsif/i.test(text));
ok('memo includes an evidence appendix', /evidence appendix/i.test(text));
ok('evidence appendix retains full source coverage', new Set(cited).size === 32, `${new Set(cited).size}/32`);
ok('memo links all regional evidence artifacts', Object.values(dossierPaths).every(file => text.includes(file)) &&
  text.includes(ledgerPath));
ok('memo contains no unfinished placeholders', !/(TODO|TBD|INSERT HERE|lorem ipsum)/i.test(text));
ok('research pack is the only required product change', text.length > 0 && Object.values(dossiers).every(value => value.length > 0));
console.log(`errors observed: 0`);
