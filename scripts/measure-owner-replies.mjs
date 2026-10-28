// Fixed Reno transcript replay against an already connected host broker. This evaluates writing,
// not tool execution or production-company state. Never log the broker credential.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';

const [label] = process.argv.slice(2);
if (!['before','after'].includes(label)) throw new Error('Pass before or after');
const endpoint = process.env.RESTLESS_EVAL_BROKER ?? 'http://127.0.0.1:7796';
if (new URL(endpoint).hostname !== '127.0.0.1') throw new Error('Use an existing local model broker');
const model = 'openai-codex/gpt-5.6-terra';
const token = readFileSync(join(process.env.HOME, '.omp/profiles/restless-model-gateway/auth-gateway.token'),'utf8').trim();
const style = readFileSync('crates/restless-engine/prompts/converse-with-owner.md','utf8');
const cases = [
    { id:'resend-account', question:'You said Resend is bundled, then said there’s no account. Which is it?',
      facts:'Restless has code for sending through Resend. It does not include a Resend account or API key. This company has no Resend account connected. A company key belongs in Company > Vault.' },
    { id:'existing-key', question:"I gave you the key, it's in the vault!",
      facts:'The company Vault contains a key named resend. The sending code requests resend.production. This mismatch has not been fixed in this snapshot. The key is already supplied; asking for it again is wrong. No email was sent. This is a Restless defect.' },
    { id:'brand-approval', question:'Is that Brand approval actually in my Inbox?',
      facts:'The Brand rule is awaiting your OK on Company > Brand. The Inbox currently has no item for it. This is a Restless defect; no approval has been given. You have no write tools in this replay and cannot claim to have filed an item.' },
    { id:'map-review', question:'Do you have a site where I can see this drawn map?',
      facts:'This fixed scenario has a concept map page at http://reno.localhost/concepts/map, previously verified by the company. Its review card has no preview link. Link the page; do not ask the reader to find an ID. This scenario is replay data, not current production state.' },
    { id:'email-revision', question:'What changed in the first S&I email?',
      facts:'The email now opens with one concrete observation about the business website and ends with one clear question. The draft is /company/outputs/si-outreach.md. No additional rewrite or email send took place in this turn.' }
];
const out = 'docs/sprints/sprint-109/evidence';
mkdirSync(out,{recursive:true});
const records=[];
for (const scenario of cases) {
    const started=Date.now();
    const response=await fetch(`${endpoint}/v1/chat/completions`, {
        method:'POST', headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},
        body:JSON.stringify({model, max_tokens:1600, messages:[
            {role:'system',content:`You are the Reno company Exec. This is a fixed transcript replay: answer from the supplied facts, with no tools and no claims to have taken actions. Append one hidden metadata comment in this format: <!--restless-intent:{"kind":"conversation","summary":"short hidden routing summary"}-->. Optional ownerNeed is only for a necessary question.\n\n${style}`},
            {role:'user',content:`Verified snapshot facts for this replay:\n${scenario.facts}\n\nYour message: ${scenario.question}`}
        ]}), signal:AbortSignal.timeout(45000)
    });
    if(!response.ok) throw new Error(`Broker returned ${response.status} for ${scenario.id}`);
    const body=await response.json();
    const reply=body.choices?.[0]?.message?.content;
    if(typeof reply !== 'string') throw new Error('Broker returned no reply');
    const visible=reply.replace(/<!--restless-(?:intent|details):[\s\S]*?-->/g,'').trim();
    records.push({...scenario, reply,visible, words:visible.split(/\s+/u).filter(Boolean).length,
        ms:Date.now()-started,usage:body.usage});
    console.log(`${label} ${scenario.id}: ${records.at(-1).words} visible words`);
    writeFileSync(join(out,`owner-replies-${label}.json`),JSON.stringify({model,
        protocol:'Fixed facts, identical five Reno prompts; one real inference per case; no tools or company writes',
        style_sha256:createHash('sha256').update(style).digest('hex'),style,records},null,2)+'\n');
}
