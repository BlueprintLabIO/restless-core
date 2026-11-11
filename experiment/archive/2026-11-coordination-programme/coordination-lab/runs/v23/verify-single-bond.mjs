import { chromium, chromiumExecutable } from '../../v2/workdir/v23-matched-single/canonical/verify-runtime.mjs';

const browser = await chromium.launch({
    executablePath: chromiumExecutable,
    headless: true,
    args: ['--no-sandbox', '--use-gl=angle', '--use-angle=swiftshader', '--ignore-gpu-blocklist', '--enable-webgl'],
});
const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
const errors = [];
page.on('console', (message) => {
    if (message.type() === 'error') errors.push(`console: ${message.text()}`);
});
page.on('pageerror', (error) => errors.push(`page: ${error.message}`));
page.on('requestfailed', (request) => errors.push(`request: ${request.url()} ${request.failure()?.errorText || ''}`));

try {
    await page.goto('http://127.0.0.1:8124/index.html', { waitUntil: 'domcontentloaded' });
    await page.waitForFunction(() => window.__game && document.querySelector('#intro'));
    await page.click('.starter-card:nth-child(1) .sc-btn');
    await page.waitForFunction(() => window.__game?.mode === 'play');

    const result = await page.evaluate(() => {
        const game = window.__game;
        const guardian = game.guardian;
        game.player.pos.set(guardian.pos.x, guardian.pos.y, guardian.pos.z + 4);
        game._beginBattle(guardian);
        game._resolveBattle('bond', {});
        const weakenedBondOpened = game.mode === 'bond' && game.bondTarget === guardian;
        game._endBond('bonded');
        return {
            weakenedBondOpened,
            mode: game.mode,
            unlocked: game.flags.prismUnlocked === true,
            guardianCaptured: guardian.captured === true,
            guardianHidden: guardian.mesh.visible === false,
            barrierHidden: game.prism.gate.barrier.visible === false,
            status: document.querySelector('#status')?.textContent || '',
        };
    });

    const passed = result.weakenedBondOpened
        && result.mode === 'play'
        && result.unlocked
        && result.guardianCaptured
        && result.guardianHidden
        && result.barrierHidden
        && /open/i.test(result.status)
        && errors.length === 0;

    console.log(JSON.stringify({ passed, result, errors }, null, 2));
    if (!passed) process.exitCode = 1;
} finally {
    await browser.close();
}
