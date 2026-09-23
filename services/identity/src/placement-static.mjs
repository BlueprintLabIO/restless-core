// Placement for one self-hosted company: its coordinates never move.
export function staticPlacement(config) {
  const coordinates = Object.freeze({
    ownerId: config.ownerId,
    planeId: config.planeId,
    companyId: config.companyId,
    cellId: config.cellId,
    coreOrigin: config.coreOrigin,
    coreHost: config.coreHost,
    ready: true,
  });
  return {
    defaultCompanyId: config.companyId,
    resolve: async (companyId) =>
      companyId === config.companyId ? coordinates : null,
  };
}
