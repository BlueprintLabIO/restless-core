<script lang="ts">
	/* The files chosen in a composer, as a row of small thumbnails above the text (S85-08). The row
	 * scrolls sideways when it overflows; each tile has its own remove button. Nothing leaves the
	 * browser until Send: an image previews from a local object URL, released when it goes. */
	import X from '@lucide/svelte/icons/x';

	let {
		files = $bindable<File[]>([]),
		disabled = false,
		progress = null
	}: {
		files?: File[];
		disabled?: boolean;
		/** While sending, how much of the upload has gone (0 to 1): drawn over each tile. */
		progress?: number | null;
	} = $props();

	const PREVIEW_TYPES = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp']);

	let previews = $state<Map<File, string>>(new Map());
	$effect(() => {
		const next = new Map<File, string>();
		for (const file of files)
			if (PREVIEW_TYPES.has(file.type)) next.set(file, URL.createObjectURL(file));
		previews = next;
		return () => {
			for (const url of next.values()) URL.revokeObjectURL(url);
		};
	});

	function extOf(name: string): string {
		const match = /\.([a-z0-9]{1,8})$/i.exec(name);
		return match ? match[1].toLowerCase() : 'file';
	}
	function fmtSize(bytes: number): string {
		if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
		return `${Math.max(1, Math.ceil(bytes / 1024))} KB`;
	}
	function removeAt(index: number) {
		files = files.filter((_, i) => i !== index);
	}
</script>

{#if files.length > 0}
	<ul class="attach-tray" aria-label="Attached files">
		{#each files as file, i (`${file.name}:${file.size}:${i}`)}
			{@const preview = previews.get(file)}
			<li class="tile" class:image={!!preview} title={`${file.name} · ${fmtSize(file.size)}`}>
				{#if preview}
					<img src={preview} alt={file.name} />
				{:else}
					<span class="kind">{extOf(file.name)}</span>
					<span class="name">{file.name}</span>
				{/if}
				{#if progress !== null}<span
						class="uploading"
						role="progressbar"
						aria-label="Uploading {file.name}"
						aria-valuemin={0}
						aria-valuemax={100}
						aria-valuenow={Math.round(progress * 100)}
						style={`--progress:${Math.round(progress * 360)}deg`}
					></span>{/if}
				<button
					type="button"
					class="remove"
					aria-label="Remove {file.name}"
					{disabled}
					onclick={() => removeAt(i)}><X size={11} strokeWidth={2.4} aria-hidden="true" /></button
				>
			</li>
		{/each}
	</ul>
{/if}

<style>
	.attach-tray {
		display: flex;
		gap: 8px;
		margin: 0;
		padding: 8px 10px 2px;
		overflow-x: auto;
		overscroll-behavior-x: contain;
		list-style: none;
		scrollbar-width: thin;
	}
	.tile {
		position: relative;
		display: flex;
		flex: none;
		flex-direction: column;
		justify-content: space-between;
		width: 56px;
		height: 56px;
		padding: 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-alt);
	}
	.tile.image {
		padding: 0;
		overflow: hidden;
	}
	.tile img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.kind {
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 600;
		text-transform: uppercase;
	}
	.name {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* Upload progress sits on the file itself: the tile dims and a ring fills (21 October). */
	.uploading {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		background: color-mix(in srgb, var(--surface) 55%, transparent);
	}
	.uploading::after {
		content: '';
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background: conic-gradient(var(--accent) var(--progress), color-mix(in srgb, var(--ink) 18%, transparent) 0);
		mask: radial-gradient(farthest-side, transparent calc(100% - 3px), #000 calc(100% - 2.5px));
		transition: background 120ms linear;
	}
	.tile:has(.uploading) .remove {
		display: none;
	}
	.remove {
		position: absolute;
		top: 3px;
		right: 3px;
		display: grid;
		place-items: center;
		width: 18px;
		height: 18px;
		padding: 0;
		border: 0;
		border-radius: 999px;
		background: color-mix(in srgb, var(--ink) 72%, transparent);
		color: var(--surface-raised);
		cursor: pointer;
	}
	.remove:hover {
		background: var(--ink);
	}
	.remove:disabled {
		cursor: not-allowed;
		opacity: 0.5;
	}
</style>
