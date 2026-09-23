// Adapt ACP's legacy session/set_model contract to the newer config-option contract.
// All other messages, notifications and permission requests pass through unchanged.
export class LegacyModels {
  #requests = new Map();
  #sessions = new Map();
  static optionId = "restless-legacy-model";

  request(message) {
    if (message?.id === undefined || typeof message.method !== "string")
      return message;
    const sessionId = message.params?.sessionId;
    if (
      message.method === "session/set_config_option" &&
      message.params?.configId === LegacyModels.optionId &&
      this.#sessions.has(sessionId)
    ) {
      const options = this.#sessions.get(sessionId);
      if (
        !options.options.some((option) => option.value === message.params.value)
      ) {
        return {
          rejected: {
            jsonrpc: "2.0",
            id: message.id,
            error: {
              code: -32602,
              message: "Harness did not advertise that model",
            },
          },
        };
      }
      this.#requests.set(message.id, {
        method: "legacy-selection",
        sessionId,
        value: message.params.value,
      });
      return {
        ...message,
        method: "session/set_model",
        params: { sessionId, modelId: message.params.value },
      };
    }
    if (
      [
        "session/new",
        "session/load",
        "session/resume",
        "session/fork",
      ].includes(message.method)
    ) {
      this.#requests.set(message.id, { method: message.method, sessionId });
    }
    return message;
  }

  response(message) {
    if (message?.id === undefined || message.method) return message;
    const request = this.#requests.get(message.id);
    if (!request) return message;
    this.#requests.delete(message.id);
    if (message.error) return message;
    if (request.method === "legacy-selection") {
      const option = {
        ...this.#sessions.get(request.sessionId),
        currentValue: request.value,
      };
      this.#sessions.set(request.sessionId, option);
      return {
        ...message,
        result: {
          configOptions: [option],
          _meta: { "restless/modelSelection": "legacy-acknowledged" },
        },
      };
    }
    const result = message.result;
    if (
      !result ||
      result.configOptions?.some(
        (option) => option.category === "model" || option.id === "model",
      )
    )
      return message;
    const sessionId = result.sessionId ?? request.sessionId;
    if (!sessionId || !Array.isArray(result.models?.availableModels))
      return message;
    const option = {
      id: LegacyModels.optionId,
      name: "Model",
      category: "model",
      type: "select",
      currentValue: result.models.currentModelId,
      options: result.models.availableModels
        .filter((model) => typeof model.modelId === "string")
        .map((model) => ({
          value: model.modelId,
          name: model.name ?? model.modelId,
        })),
    };
    this.#sessions.set(sessionId, option);
    return {
      ...message,
      result: {
        ...result,
        configOptions: [...(result.configOptions ?? []), option],
        _meta: {
          ...result._meta,
          "restless/modelSelection": "legacy-acknowledged",
        },
      },
    };
  }
}
