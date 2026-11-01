/* Real document opening, narrow header, margin actions and a failed connection. */
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { mkdir, writeFile } from 'node:fs/promises';

const origin = process.env.RESTLESS_SMOKE_ORIGIN;
const company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'));
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-document-still';
await mkdir(out, { recursive: true });
const browser = await chromium.launch();
const report = [];
async function read(path) {
	const response = await fetch(`${origin}/api/companies/${company}${path}`);
	assert(response.ok, path);
	return response.json();
}
const item = (await read('/attention')).items.find(row => row.title === 'Inbox outcome site verification');
const document = (await read('/documents?limit=100')).items
	.find(row => row.document.title === 'Inbox footer collaboration test')?.document;
assert(item && document, 'Seed real Inbox and document fixtures');
const href = `${origin}/${company}?item=${encodeURIComponent(item.id)}&preview=${encodeURIComponent(`/${company}/library/documents?document=${document.id}`)}`;
const tokenPath = `/documents/${document.id}/collaboration/token`;
let bodyStyle;
try {
	for (const width of [1440, 390]) {
		const context = await browser.newContext({ viewport: { width, height: 900 }, reducedMotion: 'reduce' });
		const page = await context.newPage();
		const errors = [];
		page.on('pageerror', error => errors.push(error.message));
		await page.route('**' + tokenPath, async route => {
			await new Promise(resolve => setTimeout(resolve, 2500));
			await route.continue(); // Delay the real grant; never fabricate a body or permission.
		});
		try {
			await page.goto(href);
			const preview = page.getByLabel('Inbox preview', { exact: true });
			await preview.locator('.document-opening').waitFor({ timeout: 30000 });
			const before = await preview.locator('.editor-head').boundingBox();
			const beforeStatus = await preview.locator('.collaboration-state').boundingBox();
			assert.equal(await preview.locator('.named-version-projection').count(), 0,
				'A connecting document does not show a differently styled saved body first');
			await page.screenshot({ path: `${out}/opening-${width}.png`, fullPage: true });
			const body = preview.getByRole('textbox', { name: 'Collaborative document body', exact: true });
			await body.waitFor({ timeout: 45000 });
			await preview.locator('.collaboration-state').filter({ hasText: 'Saved' }).waitFor();
			const after = await preview.locator('.editor-head').boundingBox();
			const afterStatus = await preview.locator('.collaboration-state').boundingBox();
			for (const dimension of ['x', 'y', 'width', 'height'])
				for (const [oldBox, newBox] of [[before, after], [beforeStatus, afterStatus]])
					assert(Math.abs(oldBox[dimension] - newBox[dimension]) <= 1, `header/status ${dimension} changed while connecting`);
			assert.equal(await preview.locator(':scope > header').count(), 0, 'There is no second Back header');
			const header = await preview.locator('.editor-head').evaluate(element => {
				const boxes = ['.document-title-field', '.collaboration-state', '.editor-actions']
					.map(selector => element.querySelector(selector).getBoundingClientRect());
				return boxes.map(box => box.y + box.height / 2);
			});
			assert(Math.max(...header) - Math.min(...header) <= 2, 'Saved and actions stay on the breadcrumb row');
			bodyStyle = await body.evaluate(element => {
				const style = getComputedStyle(element);
				return { font: style.fontFamily, size: style.fontSize, line: style.lineHeight };
			});
			const overlap = await preview.locator('.document-editor').evaluate(element => {
				const controls = element.parentElement.querySelector('.document-floats').getBoundingClientRect();
				const walker = document.createTreeWalker(element.querySelector('.native-document-body'), NodeFilter.SHOW_TEXT);
				const covered = [];
				while (walker.nextNode()) {
					if (!walker.currentNode.textContent.trim()) continue;
					const range = document.createRange(); range.selectNodeContents(walker.currentNode);
					for (const rect of range.getClientRects())
						if (rect.left < controls.right && rect.right > controls.left && rect.top < controls.bottom && rect.bottom > controls.top)
							covered.push(walker.currentNode.textContent.slice(0, 50));
				}
				return covered;
			});
			assert.deepEqual(overlap, [], 'Floating actions occupy the margin, never document text');
			await page.screenshot({ path: `${out}/settled-${width}.png`, fullPage: true });
			await preview.getByRole('button', { name: /^Comments.*and history$/ }).click();
			await preview.getByRole('link', { name: 'Back to Inbox item', exact: true }).waitFor();
			const back = preview.getByRole('button', { name: 'Back to the document', exact: true });
			if (await back.isVisible()) await back.click();
			else await preview.getByRole('button', { name: /^Comments.*and history$/ }).click();
			await preview.getByRole('link', { name: 'Back to Inbox item', exact: true }).click();
			await page.waitForURL(url => !url.searchParams.has('preview'));
			assert.equal(new URL(page.url()).searchParams.get('item'), item.id);
			assert.deepEqual(errors, []);
			report.push({ width, oneHeader: true, stableOpening: true, controlsCoverText: false, returnsToItem: true });
		} catch (error) {
			await page.screenshot({ path: `${out}/failed-${width}.png`, fullPage: true });
			throw error;
		} finally { await context.close(); }
	}
	const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	const page = await context.newPage();
	await page.goto(href);
	await page.getByLabel('Inbox preview', { exact: true }).getByRole('button', { name: 'Archive', exact: true }).waitFor();
	const archived = page.waitForResponse(response => response.request().method() === 'POST' && response.url().endsWith(`/documents/${document.id}/archive`));
	await page.getByLabel('Inbox preview', { exact: true }).getByRole('button', { name: 'Archive', exact: true }).click();
	assert((await archived).ok(), 'The floating Archive uses the actual owner action');
	assert.equal((await read(`/documents/${document.id}`)).document.status, 'archived');
	const restored = page.waitForResponse(response => response.request().method() === 'POST' && response.url().endsWith(`/documents/${document.id}/archive`));
	await page.getByRole('button', { name: 'Undo', exact: true }).click();
	assert((await restored).ok());
	assert.notEqual((await read(`/documents/${document.id}`)).document.status, 'archived');
	report.push({ floatingArchive: true, undoRestoredDocument: true });
	await context.close();
	const offline = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	const offlinePage = await offline.newPage();
	await offlinePage.route('**' + tokenPath, route => route.abort('internetdisconnected'));
	await offlinePage.goto(href);
	const preview = offlinePage.getByLabel('Inbox preview', { exact: true });
	await preview.locator('.named-version-projection').waitFor();
	const fallback = await preview.locator('.rendered-document').evaluate(element => {
		const style = getComputedStyle(element);
		return { font: style.fontFamily, size: style.fontSize, line: style.lineHeight };
	});
	assert.deepEqual(fallback, bodyStyle, 'A failed live connection retains the editor typography');
	await preview.locator('.collaboration-state').filter({ hasText: 'Read only' }).waitFor();
	await offlinePage.screenshot({ path: `${out}/read-only.png`, fullPage: true });
	report.push({ failedConnectionStyled: true, readOnlySaid: true });
	await offline.close();
	console.log('PASS cold/delayed document opening, one stable header, margin actions and styled read-only recovery');
} finally {
	await writeFile(`${out}/report.json`, JSON.stringify(report, null, 2));
	await browser.close();
}
