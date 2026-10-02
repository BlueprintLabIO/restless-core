<script lang="ts">
	import { formatMoment, formatRelative } from './time';
	let { value, fallback = '' }: { value?: Date | string | number | null; fallback?: string } =
		$props();
</script>

<time
	title={formatMoment(value)}
	datetime={value && !Number.isNaN(new Date(value).getTime())
		? new Date(value).toISOString()
		: undefined}
>
	{formatRelative(value, fallback)}
</time>
