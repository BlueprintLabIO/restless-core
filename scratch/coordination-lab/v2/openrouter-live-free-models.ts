/**
 * Scratch-only OMP registry bridge for live OpenRouter models that have not
 * reached OMP's bundled catalogue yet.
 *
 * OMP 17.2.15 parses provider-level `transport: pi-native` for built-in and
 * discovered models, but drops it while finalizing a new custom model from
 * models.yml. Registering the same two models through the extension API lets
 * the runtime registry apply the transport override and keeps credentials in
 * Restless's auth gateway.
 */
export default function registerLiveFreeModels(pi: any) {
  const gatewayPort = process.env.COORD_GATEWAY_PORT ?? "7796";
  const route = {
    baseUrl: `http://host.docker.internal:${gatewayPort}`,
    apiKey: "RESTLESS_MODEL_GATEWAY_TOKEN",
  };

  // OMP 17.2.15 returns early when a registration includes `models`, before
  // storing the transport override. Register the route first so the following
  // model registration can inherit it.
  pi.registerProvider("openrouter", {
    ...route,
    transport: "pi-native",
  });
  pi.registerProvider("openrouter", {
    ...route,
    api: "openai-completions",
    models: [
      {
        id: "stealth/ox-alpha",
        name: "Ox Alpha",
        api: "openai-completions",
        reasoning: true,
        input: ["text"],
        supportsTools: true,
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
        contextWindow: 1_048_576,
        maxTokens: 131_072,
      },
      {
        id: "z-ai/glm-5.2:free",
        name: "Z.ai GLM 5.2 (free)",
        api: "openai-completions",
        reasoning: true,
        thinking: {
          mode: "openai",
          efforts: ["minimal", "low", "medium", "high", "xhigh"],
        },
        input: ["text"],
        supportsTools: true,
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
        contextWindow: 256_000,
        maxTokens: 131_072,
      },
    ],
  });

  // CLI model resolution happens before extension registrations are drained.
  // Re-bind the already selected identity to the now transport-aware registry
  // entry before the first provider request.
  pi.on("session_start", async (_event: unknown, ctx: any) => {
    const selected = ctx.models.resolve(`${ctx.model.provider}/${ctx.model.id}`);
    if (selected) await pi.setModel(selected);
  });
}
