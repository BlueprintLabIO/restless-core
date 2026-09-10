const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const COLLABORATION_PATH = /^\/api\/companies\/([0-9a-f-]+)\/documents\/([0-9a-f-]+)\/collaboration$/;

export interface CollaborationTarget {
  readonly companyId: string;
  readonly documentId: string;
}

function requestUrl(value: string): URL {
  try {
    return new URL(value, 'http://restless-sidecar.invalid');
  } catch {
    throw new Error('collaboration request URL is invalid');
  }
}

export function parseCollaborationPath(value: string): CollaborationTarget {
  const url = requestUrl(value);
  const match = COLLABORATION_PATH.exec(url.pathname);
  if (!match || url.search || url.hash) throw new Error('collaboration request path is invalid');
  const companyId = match[1] ?? '';
  const documentId = match[2] ?? '';
  if (!CANONICAL_UUID.test(companyId) || !CANONICAL_UUID.test(documentId)) {
    throw new Error('collaboration request path must contain canonical UUIDs');
  }
  return { companyId, documentId };
}

export function parseCollaborationTarget(requestUrl: string, documentName: string): CollaborationTarget {
  const target = parseCollaborationPath(requestUrl);
  const expectedDocumentName = `${target.companyId}:${target.documentId}`;
  if (documentName !== expectedDocumentName) {
    throw new Error('collaboration route and document name must identify exact UUIDs');
  }
  return target;
}

export function collaborationDocumentName(target: CollaborationTarget): string {
  if (!CANONICAL_UUID.test(target.companyId) || !CANONICAL_UUID.test(target.documentId)) {
    throw new Error('collaboration target must identify exact UUIDs');
  }
  return `${target.companyId}:${target.documentId}`;
}
