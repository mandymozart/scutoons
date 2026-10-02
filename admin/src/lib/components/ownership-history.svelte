<script lang="ts">
	import { Badge, DataTable, type Column } from '@scutoons/ui';
	import type { OwnershipTransfer } from '@scutoons/api';
	import { minute } from '$lib/format.ts';
	import { transferKindLabel } from '$lib/claims.svelte.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AccountLink from './account-link.svelte';

	/** The ownership history of a code (`item_transfers`, newest first): claims decided, gifts, admin transfers. */
	let { items }: { items: OwnershipTransfer[] } = $props();

	const columns = $derived<Column[]>([
		{ id: 'created_at', label: m['claims.col_when']() },
		{ id: 'kind', label: m['claims.col_kind']() },
		{ id: 'from_to', label: m['claims.col_from_to']() },
		{ id: 'by', label: m['claims.col_by']() },
		{ id: 'note', label: m['claims.col_note']() }
	]);
</script>

<DataTable {columns} rows={items} rowKey={(t) => String(t.id)} emptyLabel={m['claims.history_empty']()} labels={tableLabels()}>
	{#snippet cell(t, column)}
		{#if column.id === 'created_at'}{minute(t.created_at)}
		{:else if column.id === 'kind'}<Badge tone="neutral">{transferKindLabel(t.kind)}</Badge>
			{#if t.claim_id}<a href="/claims/{encodeURIComponent(t.claim_id)}">{m['claims.open_claim']()}</a>{/if}
		{:else if column.id === 'from_to'}<AccountLink account={t.from} empty={m['claims.nobody']()} /> → <AccountLink account={t.to} />
			{#if t.item_id}<span class="muted item">{t.item_id}{t.variant ? ` · ${t.variant}` : ''}{t.qty > 1 ? ` × ${t.qty}` : ''}</span>{/if}
		{:else if column.id === 'by'}<AccountLink account={t.by} />
		{:else}<span class="note">{t.note ?? '—'}</span>{/if}
	{/snippet}
</DataTable>

<style>
	.item {
		display: block;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.note {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
</style>
