<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { Button } from '@scutoons/ui';
	import { api, ApiError } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';
	import {
		IDLE_DAYS,
		LIFETIME_DAYS,
		approveBody,
		codeFromUrl,
		formatCode,
		normalizeCode,
		type Deck,
		type PendingDeck
	} from '$lib/account/decks.ts';
	import DeckList from './deck-list.svelte';

	/**
	 * "Decks" on the account page (#180, agents/research/CYBERDECKS.md §9): every member pairs their decks here, readers
	 * included – a deck then plays with the member's access, read-only. Phone-first: the QR code on the deck's splash
	 * opens `/account/decks?deck=<code>` (the code prefilled, looked up at once); the member checks the code, picks the
	 * lifetime (six months by default, like the AI connections; optionally ending early when unused) and approves. Below:
	 * the paired decks (deck-list.svelte). The pairing link signed out comes back here after the sign-in (account-view).
	 */
	let items = $state<Deck[]>([]);
	let code = $state(codeFromUrl(page.url));
	let found = $state<PendingDeck | null>(null);
	let name = $state('');
	let days = $state<number>(LIFETIME_DAYS[0]);
	let idle = $state<number>(0);
	let working = $state(false);
	let error = $state<string | null>(null);
	let note = $state<string | null>(null);

	function fail(e: unknown) {
		note = null;
		error =
			e instanceof ApiError && e.code === 'unknown_code'
				? m['account.decks_unknown_code']()
				: e instanceof ApiError && e.status === 429
					? m['account.decks_too_many']()
					: m['account.decks_failed']();
	}

	async function load() {
		try {
			items = (await api.get<{ items: Deck[] }>('/me/decks')).items;
		} catch {
			items = []; // signed out, or the API without the decks table yet
		}
	}

	async function find(event?: SubmitEvent) {
		event?.preventDefault();
		const normalized = normalizeCode(code);
		error = null;
		note = null;
		if (!normalized) {
			error = m['account.decks_unknown_code']();
			return;
		}
		working = true;
		try {
			found = (await api.get<{ deck: PendingDeck }>(`/me/decks/pending?code=${normalized}`)).deck;
			code = formatCode(normalized);
			name = '';
		} catch (e) {
			found = null;
			fail(e);
		} finally {
			working = false;
		}
	}

	async function approve(event: SubmitEvent) {
		event.preventDefault();
		const body = approveBody(code, days, idle, name);
		if (!body) return;
		working = true;
		error = null;
		try {
			const r = await api.post<{ item: Deck }>('/me/decks/approve', body);
			items = [r.item, ...items];
			note = m['account.decks_approved']({ name: r.item.name });
			found = null;
			code = '';
		} catch (e) {
			fail(e);
		} finally {
			working = false;
		}
	}

	onMount(() => {
		void load();
		if (code) void find();
	});
</script>

<div class="decks" data-testid="decks">
	<p class="hint">{m['account.decks_hint']()}</p>

	{#if found}
		<form class="pair" onsubmit={approve} data-testid="deck-found">
			<p class="what">
				{found.hostname || formatCode(found.code)}
				<small>
					{[
						found.build && m['account.decks_build']({ build: found.build }),
						found.class && m['account.decks_class']({ class: found.class })
					]
						.filter(Boolean)
						.join(' · ')}
				</small>
			</p>
			<p class="match">{m['account.decks_match']({ code: formatCode(found.code) })}</p>
			<label for="deck-name">{m['account.decks_name']()}</label>
			<input
				id="deck-name"
				bind:value={name}
				maxlength="100"
				placeholder={found.hostname}
				autocomplete="off"
			/>
			<label for="deck-days">{m['account.decks_lifetime']()}</label>
			<select id="deck-days" bind:value={days}>
				{#each LIFETIME_DAYS as d (d)}
					<option value={d} selected={d === days}>{m[`account.decks_days_${d}`]()}</option>
				{/each}
			</select>
			<label for="deck-idle">{m['account.decks_idle']()}</label>
			<select id="deck-idle" bind:value={idle}>
				{#each IDLE_DAYS as d (d)}
					<option value={d} selected={d === idle}>
						{d === 0 ? m['account.decks_idle_never']() : m[`account.decks_days_${d}`]()}
					</option>
				{/each}
			</select>
			<div class="row">
				<Button size="sm" variant="clear" onclick={() => ((found = null), (code = ''))}>
					{m['account.decks_cancel']()}
				</Button>
				<Button size="sm" type="submit" disabled={working} data-testid="deck-approve">
					{m['account.decks_approve']()}
				</Button>
			</div>
		</form>
	{:else}
		<form class="find" onsubmit={find}>
			<label for="deck-code">{m['account.decks_code']()}</label>
			<div class="row">
				<input
					id="deck-code"
					class="code"
					bind:value={code}
					maxlength="9"
					placeholder="K7Q-42M"
					autocomplete="off"
					autocapitalize="characters"
					spellcheck="false"
					data-testid="deck-code"
				/>
				<Button size="sm" type="submit" disabled={working || !code.trim()} data-testid="deck-find">
					{m['account.decks_find']()}
				</Button>
			</div>
		</form>
	{/if}

	{#if note}<p class="hint" role="status">{note}</p>{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}

	<DeckList bind:items onnote={(text) => ((note = text), (error = null))} onerror={fail} />
</div>

<style>
	.decks,
	.pair,
	.find {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.pair,
	.find {
		gap: 0.25rem;
	}
	.pair {
		padding: 1rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	.hint,
	.match,
	.what,
	.error {
		margin: 0;
	}
	.hint {
		font-size: 0.875rem;
		color: var(--text-muted);
	}
	.what {
		display: flex;
		flex-direction: column;
		overflow-wrap: anywhere;
	}
	small {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.match {
		margin-bottom: 0.5rem;
	}
	label {
		margin-top: 0.5rem;
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.find label {
		margin-top: 0;
	}
	input,
	select {
		min-width: 0;
		min-height: var(--control-height);
		padding: 0 0.5rem;
		border: var(--border-width-regular) solid var(--border-muted);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
	}
	input:focus-visible,
	select:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
	.code {
		flex: 1;
		font-family: var(--font-mono);
		font-size: 1.25rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}
	.find .row {
		margin-top: 0;
	}
	.error {
		color: var(--state-error);
	}
</style>
