<script lang="ts">
	import { Button } from '@scutoons/ui';
	import { api } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';
	import type { Deck } from '$lib/account/decks.ts';

	/**
	 * The member's paired decks (#180, decks-group.svelte): name, build, last seen, the end – Renew, and Revoke with an
	 * in-page confirm (the deck loses access at once and shows a new code). An empty list shows nothing.
	 */
	let {
		items = $bindable(),
		onnote,
		onerror
	}: {
		items: Deck[];
		onnote: (text: string) => void;
		onerror: (error: unknown) => void;
	} = $props();

	let working = $state(false);
	let confirming = $state<string | null>(null);

	const when = (utc: string) => formatDate(new Date(`${utc.replace(' ', 'T')}Z`));
	const details = (d: Deck) =>
		[
			d.build && m['account.decks_build']({ build: d.build }),
			d.last_seen_at && m['account.decks_last_seen']({ date: when(d.last_seen_at) }),
			d.idle_days && m['account.decks_idle_days']({ days: d.idle_days })
		]
			.filter(Boolean)
			.map((part) => `${part} · `)
			.join('');

	async function renew(id: string) {
		working = true;
		try {
			const r = await api.post<{ item: Deck }>(`/me/decks/${id}/renew`, {});
			items = items.map((d) => (d.id === id ? r.item : d));
			onnote(m['account.decks_renewed']({ date: when(r.item.expires_at) }));
		} catch (e) {
			onerror(e);
		} finally {
			working = false;
		}
	}

	async function revoke(id: string) {
		working = true;
		try {
			await api.delete(`/me/decks/${id}`);
			items = items.filter((d) => d.id !== id);
			confirming = null;
		} catch (e) {
			onerror(e);
		} finally {
			working = false;
		}
	}
</script>

{#if items.length}
	<ul class="list">
		{#each items as d (d.id)}
			<li data-testid="deck-item">
				<span class="what">
					{d.name}
					<small>
						{details(d)}<span class:ended={d.expired}>
							{d.expired
								? m['account.decks_ended']({ date: when(d.expires_at) })
								: m['account.decks_ends']({ date: when(d.expires_at) })}
						</span>
					</small>
				</span>
				{#if confirming === d.id}
					<span class="actions">
						<small>{m['account.decks_revoke_confirm']()}</small>
						<Button size="sm" variant="clear" onclick={() => (confirming = null)}>
							{m['account.decks_cancel']()}
						</Button>
						<Button
							size="sm"
							onclick={() => revoke(d.id)}
							disabled={working}
							data-testid="deck-revoke-yes"
						>
							{m['account.decks_revoke_yes']()}
						</Button>
					</span>
				{:else}
					<span class="actions">
						<Button
							size="sm"
							variant="clear"
							onclick={() => renew(d.id)}
							disabled={working}
							data-testid="deck-renew"
						>
							{m['account.decks_renew']()}
						</Button>
						<Button
							size="sm"
							variant="clear"
							onclick={() => (confirming = d.id)}
							data-testid="deck-revoke"
						>
							{m['account.decks_revoke']()}
						</Button>
					</span>
				{/if}
			</li>
		{/each}
	</ul>
{/if}

<style>
	.list {
		display: flex;
		flex-direction: column;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.list li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		padding: 0.5rem 0;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.list li:first-child {
		border-top: var(--border-width-hairline) solid var(--border-subtle);
	}
	.what {
		display: flex;
		flex-direction: column;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	small {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.25rem 0.5rem;
	}
	.ended {
		color: var(--state-error);
	}
</style>
