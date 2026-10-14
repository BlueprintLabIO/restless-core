<script lang="ts">
	/* One row of the sheet as a form: each column, named by the first row, with the selected row's
	 * value to read or change. On a phone, typing into a field is easier than into a small cell.
	 * Saving writes only the fields that changed, as one change the grid can undo. */
	import type { SheetSelection } from '$lib/model/sheet-editor';

	/* A1 names, without loading the engine with the page. */
	const letters = (col: number): string =>
		(col >= 26 ? letters(Math.floor(col / 26) - 1) : '') + String.fromCharCode(65 + (col % 26));
	const cellName = (col: number, row: number) => `${letters(col)}${row + 1}`;
	const topRow = (cell: string): number | null => {
		const match = /^[A-Z]{1,3}([1-9][0-9]*)/u.exec(cell);
		return match ? Number(match[1]) - 1 : null;
	};

	let {
		getModel,
		selection,
		editable,
		onshow
	}: {
		getModel: () => any;
		selection: SheetSelection | null;
		editable: boolean;
		/** Select a cell in the grid, as the form moves between rows. */
		onshow: (worksheet: string, cell: string) => void;
	} = $props();

	interface Field {
		col: number;
		label: string;
		before: string;
		value: string;
	}
	const MAX_FIELDS = 60;
	let fields = $state<Field[]>([]);
	let row = $state(0);
	let saved = $state('');

	const at = $derived.by(() => {
		const top = selection ? topRow(selection.cell) : null;
		return selection && top !== null ? { worksheet: selection.worksheet, row: top } : null;
	});

	function read(worksheet: string, target: number) {
		const model = getModel();
		if (!model || !model.getters.getSheetIds().includes(worksheet)) {
			fields = [];
			return;
		}
		let right = 0;
		for (const id of Object.keys(model.getters.getCells(worksheet) ?? {}))
			right = Math.max(right, model.getters.getCellPosition(id).col);
		const next: Field[] = [];
		for (let col = 0; col <= Math.min(right, MAX_FIELDS - 1); col++) {
			const header = model.getters.getEvaluatedCell({ sheetId: worksheet, col, row: 0 });
			const content =
				model.getters.getCell({ sheetId: worksheet, col, row: target })?.content ?? '';
			next.push({
				col,
				label:
					target > 0 && String(header.formattedValue ?? '').trim()
						? String(header.formattedValue)
						: letters(col),
				before: content,
				value: content
			});
		}
		row = target;
		fields = next;
	}

	$effect(() => {
		const place = at;
		saved = '';
		if (place) read(place.worksheet, place.row);
		else fields = [];
	});

	const changed = $derived(fields.filter((field) => field.value !== field.before));

	function save() {
		const model = getModel();
		const place = at;
		if (!model || !place || !changed.length) return;
		const commands = changed.map((field) => ({
			type: 'UPDATE_CELL',
			sheetId: place.worksheet,
			col: field.col,
			row,
			content: field.value
		}));
		model.dispatch('RESTLESS_REPLAY', { commands });
		saved = `Saved ${changed.length === 1 ? 'one field' : `${changed.length} fields`}`;
		read(place.worksheet, row);
	}

	/* The first empty row under what the tab holds, for adding a record. */
	function addRow() {
		const model = getModel();
		const place = at;
		if (!model || !place) return;
		let bottom = 0;
		for (const id of Object.keys(model.getters.getCells(place.worksheet) ?? {}))
			bottom = Math.max(bottom, model.getters.getCellPosition(id).row);
		onshow(place.worksheet, cellName(0, bottom + 1));
	}

	function move(by: number) {
		const place = at;
		if (!place) return;
		onshow(place.worksheet, cellName(0, Math.max(0, row + by)));
	}
</script>

{#if !at}
	<p class="quiet">Select a cell in the row to see it as a form.</p>
{:else}
	<div class="row-head">
		<button type="button" class="btn small" disabled={row === 0} onclick={() => move(-1)}
			>Previous</button
		>
		<strong>Row {row + 1}</strong>
		<button type="button" class="btn small" onclick={() => move(1)}>Next</button>
	</div>
	{#if !fields.length}
		<p class="quiet">This tab is empty. Type the column names in the first row.</p>
	{/if}
	<form
		class="row-form"
		onsubmit={(event) => {
			event.preventDefault();
			save();
		}}
	>
		{#each fields as field (field.col)}
			<label>
				<span>{field.label}</span>
				<input type="text" bind:value={field.value} readonly={!editable} />
			</label>
		{/each}
		{#if editable}
			<div class="row-actions">
				<button type="submit" class="btn small primary" disabled={!changed.length}>Save</button>
				<button type="button" class="btn small" onclick={addRow}>New row</button>
				{#if saved}<span role="status">{saved}</span>{/if}
			</div>
		{/if}
	</form>
{/if}

<style>
	.row-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin-bottom: 10px;
	}
	.row-form {
		display: grid;
		gap: 10px;
	}
	label {
		display: grid;
		gap: 4px;
	}
	label span {
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-label);
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	input {
		min-width: 0;
	}
	.row-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}
	.row-actions span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.quiet {
		color: var(--text-tertiary);
	}
</style>
