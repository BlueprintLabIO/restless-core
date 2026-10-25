import assert from 'node:assert/strict';
import { test } from 'node:test';
import katex from 'katex';

test('inherited trust never enables active links or external images in model-supplied math', () => {
	const prior = Object.getOwnPropertyDescriptor(Object.prototype, 'trust');
	Object.defineProperty(Object.prototype, 'trust', {
		configurable: true,
		writable: true,
		value: true
	});
	try {
		for (const expression of [
			String.raw`\href{javascript:alert(1)}{unsafe}`,
			String.raw`\includegraphics{https://outside.restless.test/image.png}`
		]) {
			const rendered = katex.renderToString(expression, { throwOnError: false });
			assert.doesNotMatch(rendered, /<(?:a|img)\b/i);
			assert.doesNotMatch(rendered, /\b(?:href|src)\s*=/i);
		}
	} finally {
		if (prior) Object.defineProperty(Object.prototype, 'trust', prior);
		else delete Object.prototype.trust;
	}
});

test('ordinary inline, display and matrix math retain accessible rendering', () => {
	for (const expression of [
		String.raw`E=mc^2`,
		String.raw`\frac{1}{2}+\sqrt{x}`,
		String.raw`\begin{pmatrix}a&b\\c&d\end{pmatrix}`
	]) {
		for (const displayMode of [false, true]) {
			const rendered = katex.renderToString(expression, { displayMode });
			assert.match(rendered, /class="katex(?: |")/);
			assert.match(rendered, /<math\b/);
			assert.doesNotMatch(rendered, /katex-error/);
		}
	}
});
