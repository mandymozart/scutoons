<script lang="ts">
	import { Button, confirmDialog } from '@scutoons/ui';
	import { ApiError, type CatalogHistoryEntry, type CatalogKind } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, minute } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The changes of one catalog entry (`id`) or of a whole kind (no `id`: every entry + the order), newest first, from
	 * GET /admin/catalog/history; each one can be undone (POST …/history/<n>/undo – recorded as a change itself, so an
	 * undo can be undone). Changed again since → 409 changed_since → in-page "Undo anyway?" (force).
	 */
	let {
		kind,
		id,
		reload = 0,
		onundone
	}: {
		kind: CatalogKind;
		id?: string;
		/** bump to read the history again (after a save) */
		reload?: number;
		onundone: () => void;
	} = $props();

	const toaster = getToaster();
	let items = $state<CatalogHistoryEntry[] | null>(null);
	let error = $state<string | null>(null);
	let busy = $state<number | null>(null);

	$effect(() => {
		void reload;
		const params = id === undefined ? { kind, limit: 100 } : { kind, id, limit: 50 };
		error = null;
		admin
			.catalogHistory(params)
			.then((r) => (items = r.items))
			.catch((e) => (error = errorText(e)));
	});

	const ACTIONS: Record<CatalogHistoryEntry['action'], () => string> = {
		create: () => m['catalog.history_create'](),
		update: () => m['catalog.history_update'](),
		delete: () => m['catalog.history_delete'](),
		reorder: () => m['catalog.history_reorder'](),
		replace: () => m['catalog.history_replace']()
	};

	/** the top-level fields that differ between before and after (an update) */
	function changedKeys(h: CatalogHistoryEntry): string[] {
		const a = h.before,
			b = h.after;
		if (!a || !b || typeof a !== 'object' || typeof b !== 'object' || Array.isArray(a) || Array.isArray(b)) return [];
		const before = a as Record<string, unknown>,
			after = b as Record<string, unknown>;
		return [...new Set([...Object.keys(before), ...Object.keys(after)])].filter((k) => JSON.stringify(before[k]) !== JSON.stringify(after[k]));
	}

	async function undo(h: CatalogHistoryEntry) {
		busy = h.id;
		try {
			try {
				await admin.catalogUndo(h.id);
			} catch (e) {
				if (!(e instanceof ApiError && e.code === 'changed_since')) throw e;
				const force = await confirmDialog({
					title: m['catalog.undo_changed_title'](),
					message: m['catalog.undo_changed_message'](),
					confirmLabel: m['catalog.undo_anyway']()
				});
				if (!force) return;
				await admin.catalogUndo(h.id, true);
			}
			toaster.addToast({ message: m['catalog.undone'](), type: 'success' });
			onundone();
		} catch (e) {
			const extra = e instanceof ApiError ? [...(e.body?.problems ?? []), ...(e.body?.uses ?? [])] : [];
			toaster.addToast({ message: [errorText(e), ...extra].join(' · '), type: 'error' });
		} finally {
			busy = null;
		}
	}
</script>

{#if error}
	<p class="error-box" role="alert">{error}</p>
{:else if !items}
	<p class="muted">{m['shell.loading']()}</p>
{:else if items.length === 0}
	<p class="muted">{m['catalog.history_empty']()}</p>
{:else}
	<ol class="history" data-testid="catalog-history">
		{#each items as h (h.id)}
			{@const keys = h.action === 'update' ? changedKeys(h) : []}
			<li data-testid="history-row" data-action={h.action}>
				<span class="what">
					<span>{ACTIONS[h.action]?.() ?? h.action}{#if id === undefined && h.entry_id}{' · '}{h.entry_id}{/if}{#if h.undo_of}{' · '}{m['catalog.history_undo_of']({ n: h.undo_of })}{/if}</span>
					<span class="sc-caption muted">#{h.id} · {minute(h.at)} · {(h.by ?? '—').replace(/^admin:/, '')}{#if h.note}{' · '}{h.note}{/if}</span>
					{#if keys.length}<span class="sc-caption muted">{keys.join(', ')}</span>{/if}
				</span>
				<Button size="sm" variant="secondary" onclick={() => undo(h)} disabled={busy !== null} data-testid="history-undo">{busy === h.id ? '…' : m['catalog.undo']()}</Button>
			</li>
		{/each}
	</ol>
{/if}

<style>
	.history {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-075);
		padding: var(--space-05) 0;
		border-bottom: 1px solid var(--border-subtle, currentColor);
	}
	.what {
		display: flex;
		flex-direction: column;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	p {
		margin: 0;
	}
</style>
