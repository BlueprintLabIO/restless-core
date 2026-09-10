import assert from 'node:assert/strict';
import test from 'node:test';

import {
  collaborationDocumentName,
  parseCollaborationPath,
  parseCollaborationTarget,
} from '../src/target.js';
import { COMPANY_ID, DOCUMENT_ID } from './helpers.js';

const PATH = `/api/companies/${COMPANY_ID}/documents/${DOCUMENT_ID}/collaboration`;
const NAME = `${COMPANY_ID}:${DOCUMENT_ID}`;

test('the route, company, document, and Hocuspocus name have one canonical representation', () => {
  assert.deepEqual(parseCollaborationPath(PATH), { companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  assert.deepEqual(parseCollaborationTarget(`http://cell.test${PATH}`, NAME), {
    companyId: COMPANY_ID,
    documentId: DOCUMENT_ID,
  });
  assert.equal(collaborationDocumentName({ companyId: COMPANY_ID, documentId: DOCUMENT_ID }), NAME);

  const invalid: Array<readonly [string, string]> = [
    [`${PATH}/`, NAME],
    [`${PATH}?token=secret`, NAME],
    [PATH.replace(COMPANY_ID, COMPANY_ID.toUpperCase()), NAME],
    [PATH.replace('-', '%2d'), NAME],
    [PATH, DOCUMENT_ID],
    [PATH, `${DOCUMENT_ID}:${COMPANY_ID}`],
  ];
  for (const [url, name] of invalid) {
    assert.throws(() => parseCollaborationTarget(url, name));
  }
});
