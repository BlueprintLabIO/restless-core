import assert from 'node:assert/strict';
import test from 'node:test';
import { companyImageSrc, resolveCompanyImage } from './company-images.ts';

const company = 'reno';
const origin = 'https://restless.run';
const pack = '/company/outputs/final-send-pack.md';
const src = (path: string) => companyImageSrc(company, path);

/* Every way agents have named a picture, from the Reno send pack (3 November 2026) and the
 * prompts that teach them, and where the owner's view shows it from. */
test('each reference style an agent writes shows the picture or says why', () => {
	const cases: [string, string | undefined, ReturnType<typeof resolveCompanyImage>][] = [
		// The send pack: beside the document.
		[
			'final-send-pack-evidence/aleon-phone.png',
			pack,
			{
				kind: 'company',
				path: '/company/outputs/final-send-pack-evidence/aleon-phone.png',
				src: src('/company/outputs/final-send-pack-evidence/aleon-phone.png')
			}
		],
		[
			'./magnet/reno-magnet-preview.png',
			pack,
			{
				kind: 'company',
				path: '/company/outputs/magnet/reno-magnet-preview.png',
				src: src('/company/outputs/magnet/reno-magnet-preview.png')
			}
		],
		[
			'../shared/logo.png',
			'/company/outputs/pack/index.md',
			{
				kind: 'company',
				path: '/company/outputs/shared/logo.png',
				src: src('/company/outputs/shared/logo.png')
			}
		],
		// Exec's chat: the company path, as written or percent-encoded.
		[
			'/company/outputs/magnet/reno-magnet-mockup.png',
			undefined,
			{
				kind: 'company',
				path: '/company/outputs/magnet/reno-magnet-mockup.png',
				src: src('/company/outputs/magnet/reno-magnet-mockup.png')
			}
		],
		[
			'/company/outputs/My%20Mockup.png',
			undefined,
			{
				kind: 'company',
				path: '/company/outputs/My Mockup.png',
				src: src('/company/outputs/My Mockup.png')
			}
		],
		[
			'file:///company/outputs/magnet/reno-magnet-preview.png',
			undefined,
			{
				kind: 'company',
				path: '/company/outputs/magnet/reno-magnet-preview.png',
				src: src('/company/outputs/magnet/reno-magnet-preview.png')
			}
		],
		[
			'~/captures/home.png',
			undefined,
			{
				kind: 'company',
				path: '/company/home/captures/home.png',
				src: src('/company/home/captures/home.png')
			}
		],
		// A Library link copied from the cockpit, and the cockpit's own address for a company path.
		[
			`${origin}/reno/library?path=%2Fcompany%2Foutputs%2Fa.png`,
			undefined,
			{ kind: 'company', path: '/company/outputs/a.png', src: src('/company/outputs/a.png') }
		],
		[
			`${origin}/company/outputs/a.png`,
			undefined,
			{ kind: 'company', path: '/company/outputs/a.png', src: src('/company/outputs/a.png') }
		],
		// A relative path in a message reads from the company home.
		[
			'outputs/a.png',
			undefined,
			{ kind: 'company', path: '/company/outputs/a.png', src: src('/company/outputs/a.png') }
		],
		// The public web shows as it is.
		[
			'https://concepts.renostudio.au/aleon/og.png',
			undefined,
			{ kind: 'web', src: 'https://concepts.renostudio.au/aleon/og.png' }
		]
	];
	for (const [href, source, expected] of cases)
		assert.deepEqual(resolveCompanyImage(href, { company, source, origin }), expected, href);
});

test('a picture the owner cannot open says why instead of leaving a blank box', () => {
	for (const href of [
		'http://localhost:4173/shot.png',
		'http://127.0.0.1:8080/shot.png',
		'/workspace/shot.png',
		'/tmp/shot.png',
		'../../../etc/passwd',
		'file:///home/agent/shot.png',
		'javascript:alert(1)',
		'',
		undefined
	]) {
		const image = resolveCompanyImage(href, { company, source: pack, origin });
		assert.equal(image.kind, 'unavailable', String(href));
		assert.ok(image.kind === 'unavailable' && image.reason.length > 10, String(href));
	}
});
