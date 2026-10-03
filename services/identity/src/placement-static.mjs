// Fixed placement for the local account plane. Membership stays company-scoped.
import { coreRequest } from "./core-request.mjs";
export function staticPlacement(config) {
  const companies = config.companies ?? [{ companyId: config.companyId, cellId: config.cellId, companyName: config.companyName }];
  const coordinates = new Map(companies.map(company => [company.companyId, Object.freeze({
    ownerId: config.ownerId, planeId: config.planeId,
    companyId: company.companyId, cellId: company.cellId,
    coreOrigin: config.coreOrigin, coreHost: config.coreHost,
  })]));
  return {
    defaultCompanyId: config.companyId,
    resolve: async (companyId) =>
      coordinates.get(companyId) ?? null,
    // The account service can remain online while the Core host is unavailable.
    // Do not mint an entry handoff based on a hard-coded readiness claim.
    async checkReady(companyId) {
      const target = coordinates.get(companyId);
      if (!target) return false;
      try {
        const response = await coreRequest(target.coreOrigin + "/health", {
          method: "GET", headers: { Host: new URL(target.coreOrigin).host }, timeoutMs: 3000,
        });
        return response.ok && (await response.json()).status === "ok";
      } catch { return false; }
    },
  };
}
