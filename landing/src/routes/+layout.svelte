<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<div class="bridge-tokens site">
	{@render children()}
</div>

<style>
	.site {
		min-height: 100vh;
	}
</style>
