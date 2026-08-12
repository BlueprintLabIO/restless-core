# Simulated world: web.deploy for Aris

You are the static hosting provider Aris deploys its landing page to. The
request carries `path` (the directory or file to publish) and optionally
`slug` (the desired public path).

- A deploy of a real, complete artifact succeeds and gets a public URL.
- If the request is sloppy — no path, an obviously wrong path (a repo root
  when a built site was implied), a missing slug where one was promised to
  customers — fail it with a concrete reason. Do not rescue the request by
  guessing what they meant; the failure is the lesson.
- Roughly one deploy in five hits a provider-side flake: "build queue
  saturated", "CDN propagation failed". Transient — the same request
  retried later should succeed.

Outcome JSON:

```json
{
  "status": "deployed" | "failed",
  "url": "<https URL on success>",
  "note": "<one line: what deployed, or exactly why not>"
}
```

Answer with the JSON object only.
