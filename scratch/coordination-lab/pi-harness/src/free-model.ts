export interface FreeModelProof {
  id: string;
  name: string;
  checkedAt: string;
  contextLength: number;
  promptPrice: string;
  completionPrice: string;
  supportedParameters: string[];
}

interface OpenRouterModel {
  id?: unknown;
  name?: unknown;
  context_length?: unknown;
  pricing?: { prompt?: unknown; completion?: unknown };
  supported_parameters?: unknown;
  architecture?: { input_modalities?: unknown };
}

export async function proveFreeToolModel(modelId: string, signal?: AbortSignal): Promise<FreeModelProof> {
  const response = await fetch("https://openrouter.ai/api/v1/models", { signal });
  if (!response.ok) throw new Error(`OpenRouter model catalogue returned HTTP ${response.status}`);
  const body = (await response.json()) as { data?: OpenRouterModel[] };
  const model = body.data?.find((candidate) => candidate.id === modelId);
  if (!model) throw new Error(`model ${modelId} is absent from the live OpenRouter catalogue`);
  const promptPrice = String(model.pricing?.prompt ?? "");
  const completionPrice = String(model.pricing?.completion ?? "");
  const supportedParameters = Array.isArray(model.supported_parameters)
    ? model.supported_parameters.filter((item): item is string => typeof item === "string")
    : [];
  const inputModalities = Array.isArray(model.architecture?.input_modalities)
    ? model.architecture.input_modalities
    : [];
  if (promptPrice !== "0" || completionPrice !== "0") {
    throw new Error(`model ${modelId} is not free (prompt=${promptPrice}, completion=${completionPrice})`);
  }
  if (!supportedParameters.includes("tools") || !inputModalities.includes("text")) {
    throw new Error(`model ${modelId} does not advertise text input and tool support`);
  }
  return {
    id: modelId,
    name: typeof model.name === "string" ? model.name : modelId,
    checkedAt: new Date().toISOString(),
    contextLength: Number(model.context_length ?? 0),
    promptPrice,
    completionPrice,
    supportedParameters: [...supportedParameters].sort(),
  };
}
