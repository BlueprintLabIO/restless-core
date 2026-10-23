<script lang="ts">
	import Field from "$lib/ui/forms/Field.svelte";
	import ModelPicker from "$lib/primitives/ModelPicker.svelte";
	let model = $state("claude-sonnet-5-5");
	const models = [
		{ id: "claude-haiku-5-5", name: "Claude Haiku 5.5" },
		{ id: "claude-opus-5-5", name: "Claude Opus 5.5" },
		{ id: "claude-sonnet-5-5", name: "Claude Sonnet 5.5" },
		{ id: "claude-fable-5-1", name: "Claude Fable 5.1" }
	];
</script>
<div class="bridge-root" style="max-width:640px;padding:24px">
	<Field label="Model" for="m"><ModelPicker id="m" framed {models} current={model} onchoose={(m) => (model = m)} /></Field>
	<Field label="Next row" for="n"><input id="n" value="stays put" /></Field>
</div>
