/* Overlay and motion behaviour of the real cockpit chrome, on example data.
 *
 * Renders /gallery/shell (the actual AppShell, company switcher, command menu, tooltips, info tip and
 * hold-to-approve control) and checks what a person depends on and a static review cannot see: that a
 * menu leaves the way it arrived, that Escape and an outside click both take that path, that a tooltip
 * is reachable by keyboard and by pointer, that filtering is announced, and that reduced motion never
 * waits on an animation that will not run.
 *
 *   npm run dev            # in another terminal
 *   RESTLESS_REVIEW_ORIGIN=http://127.0.0.1:5173 node scripts/verify-overlays.mjs */

import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const origin = process.env.RESTLESS_REVIEW_ORIGIN ?? 'http://127.0.0.1:5173';
const url = `${origin}/gallery/shell`;
const browser = await chromium.launch(
	process.env.RESTLESS_BROWSER_EXECUTABLE
		? { executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE }
		: {}
);
const results = [];
const errors = [];
const check = (name, ok, detail = '') => {
	results.push(ok);
	console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? '  ' + detail : ''}`);
};

try {
	const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
	const page = await context.newPage();
	page.on('pageerror', (error) => errors.push(error.message));
	page.on(
		'console',
		(message) => message.type() === 'error' && errors.push(message.text().slice(0, 140))
	);
	await page.goto(url);
	await page.waitForTimeout(1500);

	/* The company switcher */
	const summary = page.locator('.company-switcher > summary');
	await summary.click();
	await page.keyboard.press('Escape');
	await page.waitForTimeout(40);
	let state = await page.evaluate(() => ({
		open: document.querySelector('.company-switcher').open,
		closing: document.querySelector('.company-switcher').dataset.closing !== undefined,
		animation: getComputedStyle(document.querySelector('.company-switcher-menu')).animationName
	}));
	check(
		'switcher plays an exit before closing (Escape)',
		state.open && state.closing && /popover-out/.test(state.animation),
		JSON.stringify(state)
	);
	await page.waitForTimeout(450);
	state = await page.evaluate(() => ({
		open: document.querySelector('.company-switcher').open,
		closing: document.querySelector('.company-switcher').dataset.closing !== undefined,
		focus: document.activeElement?.tagName
	}));
	check(
		'…then closes and returns focus to the trigger',
		!state.open && !state.closing && state.focus === 'SUMMARY',
		JSON.stringify(state)
	);
	await summary.click();
	await summary.click();
	await page.waitForTimeout(40);
	check(
		'clicking the trigger again takes the animated exit',
		await page.evaluate(
			() => document.querySelector('.company-switcher').dataset.closing !== undefined
		)
	);
	await page.waitForTimeout(450);
	await summary.click();
	await page.mouse.click(900, 600);
	await page.waitForTimeout(450);
	check(
		'an outside click closes it',
		await page.evaluate(() => !document.querySelector('.company-switcher').open)
	);

	/* The command menu */
	await page.goto(url);
	await page.waitForTimeout(1200);
	await page.keyboard.press('Control+k');
	await page.waitForSelector('dialog.command-menu[open]');
	await page.keyboard.type('person');
	const menu = await page.evaluate(() => ({
		status: document.querySelector('.command-menu [role=status]')?.textContent.trim(),
		more: document.querySelector('.command-more')?.textContent.trim()
	}));
	check('the result count is announced', menu.status === '8 results', JSON.stringify(menu));
	check(
		'truncated results say how many are hidden',
		/4 more in People/.test(menu.more ?? ''),
		menu.more
	);
	await page.keyboard.press('Escape');
	await page.waitForTimeout(40);
	state = await page.evaluate(() => {
		const dialog = document.querySelector('dialog.command-menu');
		return {
			open: dialog.open,
			closing: dialog.dataset.closing !== undefined,
			animation: getComputedStyle(dialog.querySelector('.command-panel')).animationName
		};
	});
	check(
		'the command menu plays an exit before closing (Escape)',
		state.open && state.closing && /popover-out/.test(state.animation),
		JSON.stringify(state)
	);
	await page.waitForTimeout(450);
	check(
		'…then closes',
		await page.evaluate(() => !document.querySelector('dialog.command-menu').open)
	);
	await page.keyboard.press('Control+k');
	await page.waitForSelector('dialog.command-menu[open]');
	check(
		'it reopens cleanly with the search focused',
		await page.evaluate(() => document.activeElement?.getAttribute('aria-label') === 'Search')
	);
	await page.keyboard.press('Escape');
	await page.waitForTimeout(450);
	await page.keyboard.press('Control+k');
	await page.waitForSelector('dialog.command-menu[open]');
	await page.mouse.click(5, 5);
	await page.waitForTimeout(450);
	check(
		'a backdrop click closes it',
		await page.evaluate(() => !document.querySelector('dialog.command-menu').open)
	);

	/* Tooltips */
	await page.goto(url);
	await page.waitForTimeout(1200);
	await page.locator('button[title="Archive this outcome"]').focus();
	await page.keyboard.press('Shift+Tab');
	await page.keyboard.press('Tab');
	await page.waitForTimeout(250);
	check(
		'keyboard focus shows the tooltip',
		await page.evaluate(
			() =>
				document.querySelector('.bridge-tooltip')?.matches(':popover-open') &&
				document.querySelector('.bridge-tooltip').textContent === 'Archive this outcome'
		)
	);
	await page.keyboard.press('Escape');
	await page.waitForTimeout(250);
	check(
		'Escape dismisses it without moving focus',
		await page.evaluate(
			() =>
				!document.querySelector('.bridge-tooltip').matches(':popover-open') &&
				document.activeElement?.textContent === 'Archive'
		)
	);
	await page.mouse.move(5, 500);
	await page.waitForTimeout(300);
	await page.locator('button[title="Archive this outcome"]').hover();
	await page.waitForTimeout(700);
	const box = await page.evaluate(() => {
		const rect = document.querySelector('.bridge-tooltip').getBoundingClientRect();
		return { x: rect.x + 8, y: rect.y + 6 };
	});
	await page.mouse.move(box.x, box.y, { steps: 6 });
	await page.waitForTimeout(260);
	check(
		'the tooltip stays when the pointer moves onto it (hoverable)',
		await page.evaluate(() => document.querySelector('.bridge-tooltip').matches(':popover-open'))
	);
	await page.mouse.move(700, 700);
	await page.waitForTimeout(215);
	const fading = await page.evaluate(() => ({
		inTopLayer: document.querySelector('.bridge-tooltip').matches(':popover-open'),
		opacity: +getComputedStyle(document.querySelector('.bridge-tooltip')).opacity
	}));
	check(
		'it fades out before leaving the top layer',
		fading.inTopLayer && fading.opacity < 1,
		JSON.stringify(fading)
	);
	await page.waitForTimeout(400);
	check(
		'…then leaves',
		await page.evaluate(() => !document.querySelector('.bridge-tooltip').matches(':popover-open'))
	);

	await page.locator('.info-tip').first().hover();
	await page.waitForTimeout(300);
	const inside = await page.evaluate(() => {
		const rect = document.querySelector('.info-tip-content').getBoundingClientRect();
		return { x: rect.x + 10, y: rect.y + 8 };
	});
	await page.mouse.move(inside.x, inside.y, { steps: 5 });
	await page.waitForTimeout(300);
	check(
		'the info tip stays open when the pointer moves onto it',
		await page.evaluate(() => document.querySelector('.info-tip-content').matches(':popover-open'))
	);
	await page.mouse.move(700, 700);
	await page.waitForTimeout(500);
	check(
		'…and closes after the pointer leaves both',
		await page.evaluate(() => !document.querySelector('.info-tip-content').matches(':popover-open'))
	);

	/* Hold to approve */
	const hold = page.locator('.hold-approve').first();
	const holdBox = await hold.boundingBox();
	await page.mouse.move(holdBox.x + 10, holdBox.y + 10);
	await page.mouse.down();
	await page.waitForTimeout(1200);
	await page.mouse.up();
	check(
		'completing a hold is announced',
		await page.evaluate(() =>
			[...document.querySelectorAll('.hold-approve + [role=status]')].some(
				(element) => element.textContent.trim().length > 0
			)
		)
	);

	/* A short press arms the button; a second press confirms, for people who cannot hold */
	await page.reload();
	await page.waitForTimeout(600);
	const tap = page.locator('.hold-approve').first();
	const tapBox = await tap.boundingBox();
	await page.mouse.move(tapBox.x + 10, tapBox.y + 10);
	await page.mouse.down();
	await page.waitForTimeout(150);
	await page.mouse.up();
	check(
		'a short press arms instead of approving',
		await page.evaluate(() => {
			const status = document.querySelector('.hold-approve + [role=status]');
			return /again/i.test(status?.textContent ?? '');
		})
	);
	await page.mouse.down();
	await page.waitForTimeout(150);
	await page.mouse.up();
	check(
		'…and a second press approves',
		await page.evaluate(() =>
			[...document.querySelectorAll('.hold-approve')].some((el) => el.classList.contains('done'))
		)
	);

	/* A quick click or keypress must not depend on the first animation frame. */
	for (const mode of ['pointer', 'keyboard']) {
		await page.reload();
		await page.waitForTimeout(600);
		const quick = page.locator('.hold-approve').first();
		if (mode === 'keyboard') await quick.focus();
		const activate = () => mode === 'pointer' ? quick.click() : page.keyboard.press('Enter');
		await activate();
		check(
			`a quick ${mode} activation arms without approving`,
			await quick.evaluate((element) => !element.classList.contains('done') && /again/i.test(element.getAttribute('aria-label') ?? ''))
		);
		await activate();
		check(`a second quick ${mode} activation approves`, await quick.evaluate((element) => element.classList.contains('done')));
	}

	/* Reduced motion: nothing waits on an animation that will not run */
	const reduced = await browser.newContext({
		viewport: { width: 1440, height: 900 },
		reducedMotion: 'reduce'
	});
	const quiet = await reduced.newPage();
	await quiet.goto(url);
	await quiet.waitForTimeout(1200);
	await quiet.locator('.company-switcher > summary').click();
	await quiet.keyboard.press('Escape');
	await quiet.waitForTimeout(80);
	check(
		'reduced motion: the switcher closes at once',
		await quiet.evaluate(() => !document.querySelector('.company-switcher').open)
	);
	await quiet.keyboard.press('Control+k');
	await quiet.waitForSelector('dialog.command-menu[open]');
	await quiet.keyboard.press('Escape');
	await quiet.waitForTimeout(80);
	check(
		'reduced motion: the command menu closes at once',
		await quiet.evaluate(() => !document.querySelector('dialog.command-menu').open)
	);

	const proof = process.env.RESTLESS_OVERLAY_PROOF_DIR;
	if (proof) {
		mkdirSync(proof, { recursive: true });
		for (const [label, viewport] of [
			['desktop', { width: 1440, height: 900 }],
			['mobile', { width: 375, height: 812 }]
		]) {
			await page.setViewportSize(viewport);
			for (const fixture of ['shell', 'portfolio']) {
				await page.goto(`${origin}/gallery/${fixture}`);
				await page.waitForTimeout(600);
				await page.screenshot({ path: join(proof, `${fixture}-${label}.png`), fullPage: true });
			}
		}
		writeFileSync(join(proof, 'results.json'), JSON.stringify({ passed: results.filter(Boolean).length, total: results.length, errors }, null, 2));
	}
} finally {
	await browser.close();
}

console.log(
	`\n${results.filter(Boolean).length}/${results.length} passed${errors.length ? '; console errors: ' + JSON.stringify(errors.slice(0, 4)) : ''}`
);
if (results.some((ok) => !ok) || errors.length) process.exit(1);
