import { request as httpRequest } from "node:http";
import { request as httpsRequest } from "node:https";
// .localhost is reserved for loopback. Browsers already resolve it that way;
// Node's system resolver may not. Public HTTPS destinations keep normal TLS checks.
export function coreRequest(url, options) {
  const target = new URL(url),
    request = target.protocol === "https:" ? httpsRequest : httpRequest;
  return new Promise((resolve, reject) => {
    const connection = request(
      target,
      {
        method: options.method,
        headers: options.headers,
        ...(target.hostname.endsWith(".localhost")
          ? {
              lookup: (_host, settings, done) =>
                settings.all
                  ? done(null, [{ address: "127.0.0.1", family: 4 }])
                  : done(null, "127.0.0.1", 4),
            }
          : {}),
      },
      (response) => {
        let size = 0;
        const chunks = [];
        response.on("data", (chunk) => {
          size += chunk.length;
          if (size > 65536)
            connection.destroy(Error("Core response exceeded its bound"));
          else chunks.push(chunk);
        });
        response.on("error", reject);
        response.on("end", () =>
          resolve({
            ok: response.statusCode >= 200 && response.statusCode < 300,
            json: async () => JSON.parse(Buffer.concat(chunks).toString()),
          }),
        );
      },
    );
    connection.setTimeout(10000, () =>
      connection.destroy(Error("Core did not respond")),
    );
    connection.on("error", reject);
    connection.end(options.body);
  });
}
