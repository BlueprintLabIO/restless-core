<script lang="ts">
	/* The composer's attach control: a 📎 button backed by a real file input,
	 * plus chips for the chosen files. Nothing leaves the browser until Send. */

	let {
		files = $bindable<File[]>([]),
		disabled = false
	}: {
		files?: File[];
		disabled?: boolean;
	} = $props();

	let inputEl = $state<HTMLInputElement | undefined>();

	/* The bound list is the truth, not the input: dropped and pasted files live only there, and
	 * each pick adds to them. The input is cleared so the same file can be picked again. */
	function sync() {
		const picked = Array.from(inputEl?.files ?? []);
		if (picked.length) files = [...files, ...picked];
		if (inputEl) inputEl.value = '';
	}

	function removeAt(index: number) {
		files = files.filter((_, i) => i !== index);
	}

	function fmtSize(bytes: number): string {
		if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
		return `${Math.max(1, Math.ceil(bytes / 1024))} KB`;
	}
</script>

<div class="attachment-picker">
	{#if files.length > 0}
		<div class="attach-chips">
			{#each files as file, i (`${file.name}:${i}`)}
				<span class="attach-chip" title={`${file.name} · ${fmtSize(file.size)}`}>
					<span class="attach-chip-name">{file.name}</span>
					<button
						type="button"
						class="attach-x"
						aria-label="Remove {file.name}"
						onclick={() => removeAt(i)}>×</button
					>
				</span>
			{/each}
		</div>
	{/if}
	<button
		class="attach-btn"
		class:off={disabled}
		type="button"
		title="Attach files (up to 6, 5 MB each)"
		aria-label="Attach files (up to 6, 5 MB each)"
		{disabled}
		onclick={() => inputEl?.click()}
	>
		<input
			bind:this={inputEl}
			type="file"
			name="attachments"
			multiple
			{disabled}
			onchange={sync}
			hidden
			tabindex="-1"
		/>
		<svg viewBox="0 0 16 16" width="16" height="16" fill="none" aria-hidden="true">
			<path
				d="M13.5 7.6 8.9 12.2a3.54 3.54 0 0 1-5-5l5.3-5.3a2.36 2.36 0 0 1 3.34 3.34l-5.3 5.3a1.18 1.18 0 0 1-1.67-1.67l4.6-4.6"
				stroke="currentColor"
				stroke-width="1.3"
				stroke-linecap="round"
				stroke-linejoin="round"
			/>
		</svg>
	</button>
</div>
