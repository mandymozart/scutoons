<script lang="ts" module>
	export interface Column {
		/** field id; also the sort field when `sortable` */
		id: string;
		label: string;
		sortable?: boolean;
		align?: 'start' | 'end';
		/** CSS width, e.g. `8rem` */
		width?: string;
		/** hide the column below 1600px (the tools' type is 24px from 768px, DESIGN.md "Brutalist tools") */
		secondary?: boolean;
	}

	export interface DataTableLabels {
		selectAll: string;
		selectRow: (key: string) => string;
		/** e.g. `3 selected` */
		selected: (count: number) => string;
		clearSelection: string;
		sortedAscending: string;
		sortedDescending: string;
	}

	/** Next sort value when a header is activated: field → -field → field (new fields start with `defaultDir`) */
	export function nextSort(current: string | null | undefined, field: string, defaultDir: 'asc' | 'desc' = 'asc'): string {
		if (current === field) return `-${field}`;
		if (current === `-${field}`) return field;
		return defaultDir === 'desc' ? `-${field}` : field;
	}
</script>

<script lang="ts" generics="T">
	import type { Snippet } from 'svelte';

	/**
	 * Data table (admin UI pattern): sortable headers, bulk select + actions, keyboard-reachable rows, loading/empty
	 * states. Server-side lists: pass `sort` (`field` | `-field`) and `sortHref(field)` so the headers are links that
	 * change the URL (the app's `load` refetches) – or `onsort` for client-side sorting. Pair it with `Pager`.
	 */
	let {
		columns,
		rows,
		rowKey,
		cell,
		sort = null,
		sortHref,
		onsort,
		defaultSortDir = {},
		selectable = false,
		selected = $bindable([]),
		bulk,
		onrowclick,
		activeKey = null,
		rowClass,
		loading = false,
		empty,
		emptyLabel = 'Nothing found.',
		caption,
		labels = {
			selectAll: 'Select all rows',
			selectRow: (key: string) => `Select ${key}`,
			selected: (count: number) => `${count} selected`,
			clearSelection: 'Clear selection',
			sortedAscending: 'sorted ascending',
			sortedDescending: 'sorted descending'
		}
	}: {
		columns: Column[];
		rows: T[];
		rowKey: (row: T) => string;
		/** renders one cell; default = the field as text */
		cell?: Snippet<[T, Column]>;
		/** current sort, `field` or `-field` */
		sort?: string | null;
		/** URL for a header (server-side, URL-driven sorting) */
		sortHref?: (sort: string) => string;
		/** callback for a header (client-side sorting) */
		onsort?: (sort: string) => void;
		/** first direction per field when it is not sorted yet (e.g. counts: desc) */
		defaultSortDir?: Record<string, 'asc' | 'desc'>;
		selectable?: boolean;
		/** keys of the selected rows */
		selected?: string[];
		/** bulk actions for the selected keys, shown in a bar above the table while something is selected */
		bulk?: Snippet<[string[]]>;
		/** row activation (click, Enter): e.g. open the detail drawer */
		onrowclick?: (row: T) => void;
		/** key of the highlighted row (the open detail) */
		activeKey?: string | null;
		rowClass?: (row: T) => string;
		loading?: boolean;
		empty?: Snippet;
		emptyLabel?: string;
		caption?: string;
		labels?: DataTableLabels;
	} = $props();

	const keys = $derived(rows.map(rowKey));
	const allSelected = $derived(keys.length > 0 && keys.every((k) => selected.includes(k)));
	const someSelected = $derived(!allSelected && keys.some((k) => selected.includes(k)));

	function toggleAll() {
		if (allSelected) selected = selected.filter((k) => !keys.includes(k));
		else selected = [...new Set([...selected, ...keys])];
	}

	function toggle(key: string) {
		selected = selected.includes(key) ? selected.filter((k) => k !== key) : [...selected, key];
	}

	function direction(column: Column): 'ascending' | 'descending' | undefined {
		if (sort === column.id) return 'ascending';
		if (sort === `-${column.id}`) return 'descending';
		return undefined;
	}

	function rowKeydown(event: KeyboardEvent, row: T) {
		if (event.target !== event.currentTarget) return;
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			onrowclick?.(row);
		}
	}

	function rowClick(event: MouseEvent, row: T) {
		// links, buttons and checkboxes inside the row do their own thing
		if ((event.target as HTMLElement).closest('a, button, input, select, textarea, label')) return;
		onrowclick?.(row);
	}

	const text = (row: T, id: string) => {
		const value = (row as Record<string, unknown>)[id];
		return value === null || value === undefined || value === '' ? '—' : String(value);
	};
</script>

<div class="sc-data-table" aria-busy={loading}>
	{#if selectable && selected.length > 0}
		<div class="bulk" role="region" aria-label={labels.selected(selected.length)}>
			<strong>{labels.selected(selected.length)}</strong>
			{#if bulk}{@render bulk(selected)}{/if}
			<button type="button" class="link" onclick={() => (selected = [])}>{labels.clearSelection}</button>
		</div>
	{/if}
	<div class="scroll">
		<table>
			{#if caption}<caption>{caption}</caption>{/if}
			<thead>
				<tr>
					{#if selectable}
						<th class="select" scope="col">
							<input
								type="checkbox"
								aria-label={labels.selectAll}
								checked={allSelected}
								indeterminate={someSelected}
								onchange={toggleAll}
							/>
						</th>
					{/if}
					{#each columns as column (column.id)}
						{@const dir = direction(column)}
						<th
							scope="col"
							class:end={column.align === 'end'}
							class:secondary={column.secondary}
							style:width={column.width}
							aria-sort={column.sortable ? (dir ?? 'none') : undefined}
						>
							{#if column.sortable && (sortHref || onsort)}
								{@const next = nextSort(sort, column.id, defaultSortDir[column.id])}
								{#if sortHref}
									<a class="sort" href={sortHref(next)} data-sveltekit-noscroll data-sveltekit-keepfocus>
										{column.label}<span class="arrow" aria-hidden="true">{dir === 'ascending' ? '↑' : dir === 'descending' ? '↓' : ''}</span>
										{#if dir}<span class="sr-only">, {dir === 'ascending' ? labels.sortedAscending : labels.sortedDescending}</span>{/if}
									</a>
								{:else}
									<button type="button" class="sort" onclick={() => onsort?.(next)}>
										{column.label}<span class="arrow" aria-hidden="true">{dir === 'ascending' ? '↑' : dir === 'descending' ? '↓' : ''}</span>
										{#if dir}<span class="sr-only">, {dir === 'ascending' ? labels.sortedAscending : labels.sortedDescending}</span>{/if}
									</button>
								{/if}
							{:else}
								{column.label}
							{/if}
						</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each rows as row, i (keys[i])}
					{@const key = keys[i]}
					<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
					<tr
						class={rowClass?.(row) ?? ''}
						class:clickable={!!onrowclick}
						class:active={activeKey === key}
						class:checked={selectable && selected.includes(key)}
						tabindex={onrowclick ? 0 : undefined}
						onclick={onrowclick ? (event) => rowClick(event, row) : undefined}
						onkeydown={onrowclick ? (event) => rowKeydown(event, row) : undefined}
					>
						{#if selectable}
							<td class="select">
								<input
									type="checkbox"
									aria-label={labels.selectRow(key)}
									checked={selected.includes(key)}
									onchange={() => toggle(key)}
								/>
							</td>
						{/if}
						{#each columns as column (column.id)}
							<td class:end={column.align === 'end'} class:secondary={column.secondary}>
								{#if cell}{@render cell(row, column)}{:else}{text(row, column.id)}{/if}
							</td>
						{/each}
					</tr>
				{:else}
					{#if !loading}
						<tr class="empty">
							<td colspan={columns.length + (selectable ? 1 : 0)}>
								{#if empty}{@render empty()}{:else}{emptyLabel}{/if}
							</td>
						</tr>
					{/if}
				{/each}
			</tbody>
		</table>
	</div>
</div>

<style>
	.sc-data-table {
		position: relative;
	}
	.sc-data-table[aria-busy='true'] tbody {
		opacity: 0.5;
	}
	.scroll {
		overflow-x: auto;
		border: var(--border-width-regular) solid var(--ink);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-variant-numeric: tabular-nums;
	}
	caption {
		text-align: left;
		padding: var(--space-075) var(--space-1);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	/* DESIGN.md "Brutalist tools": roomy rows, ink rules (no grey lines), uppercase bold headers */
	th,
	td {
		padding: var(--space-075) var(--space-1);
		text-align: left;
		vertical-align: top;
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	th {
		position: sticky;
		top: 0;
		background: var(--paper);
		border-bottom: var(--border-width-regular) solid var(--ink);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-bold);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		white-space: nowrap;
		vertical-align: middle;
	}
	.end {
		text-align: right;
	}
	.select {
		width: 1.25rem;
	}
	.sort {
		display: inline-flex;
		gap: var(--space-025);
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-decoration: none;
		cursor: pointer;
	}
	.sort:hover {
		text-decoration: underline;
	}
	.arrow {
		display: inline-block;
		min-width: 0.8em;
	}
	tbody tr:last-child td {
		border-bottom: 0;
	}
	tr.clickable {
		cursor: pointer;
	}
	tr.clickable:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	tr.checked {
		background: var(--surface-hover);
	}
	/* selected = inverted */
	tr.active {
		background: var(--ink);
		color: var(--paper);
	}
	tr.active :global(a) {
		color: inherit;
	}
	tr:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	tr.empty td {
		padding: var(--space-15) var(--space-075);
		color: var(--text-muted);
	}
	.bulk {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05) var(--space-1);
		padding: var(--space-05) var(--space-075);
		background: var(--ink);
		color: var(--paper);
	}
	.link {
		margin-left: auto;
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	input[type='checkbox'] {
		width: 1.25rem;
		height: 1.25rem;
		margin: 0;
		accent-color: var(--ink);
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	/* the reader's type size (24px root from 768px) needs the room: secondary columns only from 1600px */
	@media (max-width: 1599px) {
		.secondary {
			display: none;
		}
	}
</style>
