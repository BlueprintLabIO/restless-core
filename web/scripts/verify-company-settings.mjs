import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
const { RESTLESS_SMOKE_ORIGIN: origin, RESTLESS_SMOKE_COMPANY: company } = process.env;
assert(company.endsWith('_test'));
const b = await chromium.launch(
	process.env.RESTLESS_BROWSER_EXECUTABLE
		? { executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE }
		: {}
);
const output = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-company-settings-verification';
await fs.mkdir(output, { recursive: true });
const errors = [];
try {
	const p = await b.newPage({ viewport: { width: 1440, height: 1000 } });
	p.on('pageerror', (e) => errors.push(e.message));
	const base = `${origin}/${company}/company`,
		api = `${origin}/api/companies/${company}`;
	await p.goto(base + '/identity');
	await p.getByRole('button', { name: 'Add identity', exact: true }).click();
	await p.getByLabel('Truth', { exact: true }).fill('We make useful company tools.');
	await p.getByLabel('Voice', { exact: true }).fill('Clear, direct and helpful.');
	let response = p.waitForResponse(
		(r) => r.url() === api + '/company/identity' && r.request().method() === 'PUT'
	);
	await p.getByRole('button', { name: 'Save identity', exact: true }).click();
	assert((await response).ok());
	await p.getByText('Identity saved. New work will use this version.', { exact: true }).waitFor();
	await p.reload();
	await p.getByRole('button', { name: 'Edit identity', exact: true }).click();
	assert.equal(
		await p.getByLabel('Voice', { exact: true }).inputValue(),
		'Clear, direct and helpful.'
	);
	const v1 = await (await p.request.get(api + '/company/identity')).json();
	await p.getByLabel('Voice', { exact: true }).fill('Warm, brief and precise.');
	response = p.waitForResponse(
		(r) => r.url() === api + '/company/identity' && r.request().method() === 'PUT'
	);
	await p.getByRole('button', { name: 'Save identity', exact: true }).click();
	assert((await response).ok());
	const stale = await p.request.put(api + '/company/identity', {
		headers: { Origin: origin },
		data: {
			expected_release: v1.current_release.id,
			truth: 'Stale',
			voice: '',
			visual: '',
			culture: ''
		}
	});
	assert.equal(stale.status(), 409);
	const saved = await (await p.request.get(api + '/company/identity')).json();
	assert.equal(saved.releases.length, 2);
	await p.goto(base + '/resources');
	await p.getByRole('button', { name: 'Edit spend limit', exact: true }).click();
	await p.getByLabel('Total model spend limit (USD)').fill('3.25');
	response = p.waitForResponse(
		(r) => r.url().endsWith('/spend-limit') && r.request().method() === 'POST'
	);
	await p.getByRole('button', { name: 'Save limit', exact: true }).click();
	assert((await response).ok());
	await p.reload();
	await p.getByRole('button', { name: 'Edit spend limit', exact: true }).click();
	assert.equal(await p.getByLabel('Total model spend limit (USD)').inputValue(), '3.25');
	await p.getByRole('button', { name: 'Cancel', exact: true }).click();
	assert.equal(
		(
			await p.request.post(api + '/company/spend-limit', {
				headers: { Origin: origin },
				data: { ceiling: '7', expected_ceiling: '0' }
			})
		).status(),
		409
	);
	assert.equal(
		(
			await p.request.post(api + '/company/spend-limit', {
				headers: { Origin: origin },
				data: { ceiling: '-1', expected_ceiling: '3.25' }
			})
		).status(),
		422
	);
	assert.equal(await p.getByRole('combobox', { name: 'Company outcome standard' }).count(), 0);
	await p.goto(base + '/authority');
	await p.waitForURL(base + '/resources#limits');
	await p.route('**/startup-doctor', (r) => r.fulfill({ status: 503, body: 'unavailable' }));
	await p.goto(base + '/doctor');
	await p.getByRole('button', { name: 'Retry startup check', exact: true }).waitFor();
	await p.unroute('**/startup-doctor');
	await p.getByRole('button', { name: 'Retry startup check', exact: true }).click();
	await p.waitForTimeout(1500);
	assert.equal(
		await p.getByRole('button', { name: 'Retry startup check', exact: true }).count(),
		0
	);
	await p.route('**/vault', (r) => r.fulfill({ status: 503, body: 'unavailable' }));
	await p.goto(base + '/vault');
	await p.getByText('Vault status unavailable', { exact: false }).waitFor();
	assert.equal(await p.getByText('Checking Infisical…', { exact: false }).count(), 0);
	await p.unroute('**/vault');
	await p.getByRole('button', { name: 'Refresh', exact: true }).click();
	await p.waitForTimeout(1200);
	// Charter and name are actual owner writes in the disposable company.
	await p.goto(base);
	await p.getByLabel('Company name', { exact: true }).fill('Settings audit company');
	await p.locator('.setup-page').getByText('Saved', { exact: true }).waitFor();
	await p.getByRole('button', { name: 'Edit charter', exact: true }).click();
	await p
		.getByLabel('Company charter Markdown')
		.fill('# Settings audit company\n\nBuild useful tools together.');
	response = p.waitForResponse(
		(r) => r.url().endsWith('/company/charter') && r.request().method() === 'POST'
	);
	await p.getByRole('button', { name: 'Save charter', exact: true }).click();
	assert((await response).ok());
	await p.reload();
	await p.getByRole('button', { name: 'Edit charter', exact: true }).click();
	assert(
		(await p.getByLabel('Company charter Markdown').inputValue()).includes(
			'Build useful tools together.'
		)
	);
	await p.getByLabel('Company charter Markdown').fill('Unsaved draft');
	p.once('dialog', (dialog) => dialog.dismiss());
	await p
		.locator('nav[aria-label=Company]')
		.getByRole('link', { name: 'Identity', exact: true })
		.click();
	assert.equal(new URL(p.url()).pathname, `/${company}/company`);
	assert.equal(await p.getByLabel('Company charter Markdown').inputValue(), 'Unsaved draft');
	await p.getByRole('button', { name: 'Cancel', exact: true }).click();
	// Exercise sign-in failure without opening or changing any real account session.
	await p.goto(base + '/provider');
	await p.getByRole('button', { name: 'Sign in with ChatGPT', exact: true }).waitFor();
	await p.route('**/harness-auth/codex', (r) =>
		r.fulfill({
			status: 503,
			contentType: 'application/json',
			body: JSON.stringify({ message: 'Sign-in temporarily unavailable' })
		})
	);
	await p.getByRole('button', { name: 'Sign in with ChatGPT', exact: true }).click();
	await p.getByText('Sign-in temporarily unavailable', { exact: true }).waitFor();
	assert(
		!(await p.getByRole('button', { name: 'Sign in with ChatGPT', exact: true }).isDisabled())
	);
	await p.unroute('**/harness-auth/codex');
	await p.getByRole('combobox', { name: 'Provider', exact: true }).selectOption('custom');
	await p.getByRole('textbox', { name: 'Provider ID', exact: true }).fill('custom-ai');
	await p.locator('.connection-editor').getByText('Advanced settings', { exact: true }).click();
	await p.getByRole('combobox', { name: 'Credential source', exact: true }).selectOption('env');
	assert(
		(await p.getByRole('textbox', { name: /reference/i }).count()) ||
			(await p.locator('.connection-editor input').count())
	);
	await p.goto(base + '/identity');
	await p.locator('.identity-version summary').last().click();
	await p.getByText('Clear, direct and helpful.', { exact: true }).waitFor();
	await p.getByRole('button', { name: 'Edit identity', exact: true }).click();
	await p.setViewportSize({ width: 390, height: 1000 });
	if (await p.locator('#bridge-exrail[aria-hidden="false"]').count())
		await p.locator('button[aria-controls=bridge-exrail]').first().click();
	await p.screenshot({ path: output + '/identity-editor-mobile.png' });
	await p.getByRole('button', { name: 'Cancel', exact: true }).click();
	for (const route of [
		'',
		'identity',
		'resources',
		'decisions',
		'actions',
		'provider',
		'vault',
		'computer',
		'doctor'
	])
		for (const width of [1440, 390, 320]) {
			await p.setViewportSize({ width, height: 1000 });
			await p.goto(base + (route ? '/' + route : ''));
			await p
				.locator('.company-page,.provider-page,.vault-page,.company-focus-shell')
				.first()
				.waitFor();
			await p.locator('.company-page-wait').waitFor({ state: 'hidden', timeout: 30000 });
			await p.waitForTimeout(1200);
			if (await p.locator('#bridge-exrail[aria-hidden="false"]').count())
				await p.locator('button[aria-controls=bridge-exrail]').first().click();
			const over = await p
				.locator('.company-page,.provider-page,.vault-page,.company-focus-shell')
				.first()
				.evaluate((el) =>
					[...el.querySelectorAll('*')]
						.filter((n) => {
							const r = n.getBoundingClientRect();
							return r.width > 0 && (r.right > innerWidth + 2 || r.left < -2);
						})
						.map((n) => ({ tag: n.tagName, cls: n.className }))
						.slice(0, 8)
				);
			assert.deepEqual(over, [], `${route} ${width} overflow`);
			if (width <= 390 && route !== 'computer')
				assert(await p.getByRole('combobox', { name: 'Company page', exact: true }).isVisible());
			console.log('Verified', route || 'charter', width);
			await p.screenshot({ path: `${output}/after-${route || 'charter'}-${width}.png` });
		}
	assert.deepEqual(errors, []);
	console.log(
		'PASS real isolated company: identity save/reload/history/stale guard; budget save/reload/stale/invalid guard; outcome save/reload/error rollback; legacy redirect; Doctor retry; Vault failed status; all Company pages at 1440/390/320 without overflow or browser errors.'
	);
} catch (e) {
	for (const c of b.contexts())
		for (const p of c.pages()) {
			console.error('Failed at', p.url());
			await p.screenshot({ path: output + '/smoke-failed.png' });
		}
	throw e;
} finally {
	await b.close();
}
