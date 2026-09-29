<script lang="ts">
	/**
	 * Raw record inspector (admin UI pattern, on every detail page): the record as a key/value table, or as JSON.
	 * Nested values are shown as compact JSON in the table.
	 */
	let {
		record,
		labels = { fields: 'Fields', json: 'JSON', key: 'Key', value: 'Value', view: 'View' }
	}: {
		record: unknown;
		labels?: { fields: string; json: string; key: string; value: string; view: string };
	} = $props();

	let mode = $state<'fields' | 'json'>('fields');

	const entries = $derived(
		record && typeof record === 'object' && !Array.isArray(record)
			? Object.entries(record as Record<string, unknown>)
			: [['', record] as [string, unknown]]
	);
	const json = $derived(JSON.stringify(record, null, 2));

	const show = (value: unknown) =>
		value === null ? 'null' : value === undefined ? '' : typeof value === 'object' ? JSON.stringify(value) : String(value);
</script>

<div class="sc-inspector">
	<div class="switch" role="group" aria-label={labels.view}>
		<button type="button" aria-pressed={mode === 'fields'} onclick={() => (mode = 'fields')}>{labels.fields}</button>
		<button type="button" aria-pressed={mode === 'json'} onclick={() => (mode = 'json')}>{labels.json}</button>
	</div>
	{#if mode === 'fields'}
		<table>
			<thead>
				<tr><th scope="col">{labels.key}</th><th scope="col">{labels.value}</th></tr>
			</thead>
			<tbody>
				{#each entries as [key, value] (key)}
					<tr>
						<th scope="row">{key}</th>
						<td class:null={value === null}>{show(value)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{:else}
		<pre>{json}</pre>
	{/if}
</div>

<style>
	.sc-inspector {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.switch {
		display: flex;
	}
	.switch button {
		padding: 0.2em 0.8em;
		border: var(--border-width-hairline) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}
	.switch button + button {
		border-left: 0;
	}
	.switch button[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		border: var(--border-width-hairline) solid var(--border-subtle);
	}
	th,
	td {
		padding: var(--space-025) var(--space-05);
		text-align: left;
		vertical-align: top;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
		overflow-wrap: anywhere;
	}
	thead th {
		color: var(--text-muted);
		font-weight: var(--font-weight-regular);
	}
	tbody th {
		width: 30%;
		background: var(--surface-hover);
		font-weight: var(--font-weight-regular);
		white-space: nowrap;
	}
	.null {
		color: var(--text-muted);
	}
	pre {
		margin: 0;
		padding: var(--space-075);
		max-height: 30rem;
		overflow: auto;
		background: var(--surface-panel);
		font: inherit;
		font-size: var(--type-caption-size);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
</style>
