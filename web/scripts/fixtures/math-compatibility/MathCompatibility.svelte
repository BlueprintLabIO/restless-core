<script>
	import { Streamdown } from 'svelte-streamdown';
	import MathRenderer from 'svelte-streamdown/math';

	const initial = String.raw`Inline math: $E=mc^2$.

$$\frac{1}{2}+\sqrt{x}$$

$$\begin{pmatrix}a&b\\c&d\end{pmatrix}$$

Streaming: $\frac{1}`;
	let content = $state(initial);
	let streaming = $state(true);
</script>

<h1>Math rendering</h1>
<Streamdown
	{content}
	static={!streaming}
	parseIncompleteMarkdown={streaming}
	renderHtml={false}
	components={{ math: MathRenderer }}
	animation={{ enabled: false }}
/>
<button
	onclick={() => {
		content = initial + String.raw`{2}$`;
		streaming = false;
	}}>Complete stream</button
>

<style>
	:global(body) {
		margin: 20px;
	}
	:global(main) {
		max-width: 700px;
		margin: auto;
	}
</style>
