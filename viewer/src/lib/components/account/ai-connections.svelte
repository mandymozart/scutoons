<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Icon } from '@scutoons/ui';
	import { api } from '$lib/api.ts';
	import { API_URL } from '$lib/config.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { formatDate } from '$lib/utils/date.ts';

	/**
	 * "AI connections" on the account page (agents/MCP.md §3, approved screen 2026-10-09): the member's personal tokens
	 * for their own Claude (the SCUTOONS MCP server) – name, created, last used, Revoke (in-page confirm); "New
	 * connection" asks for a name and shows the token ONCE with Copy and the setup steps for Claude Desktop (the
	 * extension) or Claude Code (one command). Members only: readers don't see the section (`member` from the API).
	 * Hidden until the API answers; an empty list shows only the button.
	 */
	interface Connection {
		id: string;
		name: string;
		created_at: string;
		last_used_at: string | null;
		expires_at: string | null;
		expired: boolean;
	}

	let member = $state(false);
	let items = $state<Connection[]>([]);
	let adding = $state(false);
	let name = $state('');
	let working = $state(false);
	let error = $state<string | null>(null);
	let confirming = $state<string | null>(null);
	/** the token just created – shown this once */
	let fresh = $state<{ id: string; token: string; name: string } | null>(null);
	let app = $state<'desktop' | 'code'>('desktop');
	let copied = $state(false);
	/** the line after a renew */
	let renewed = $state<string | null>(null);

	const when = (utc: string) => formatDate(new Date(`${utc.replace(' ', 'T')}Z`));
	const command = $derived(
		`claude mcp add scutoons --scope user -e SCUTOONS_API=${API_URL} -e SCUTOONS_TOKEN=${fresh?.token ?? 'sct_…'} -- node <folder>/scutoons-mcp.mjs`
	);

	async function load() {
		try {
			const r = await api.get<{ member: boolean; items: Connection[] }>('/me/tokens');
			member = r.member;
			items = r.items;
		} catch {
			member = false; // signed out, or the API without the tokens table yet
		}
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (!name.trim()) return;
		working = true;
		error = null;
		try {
			const r = await api.post<{ token: string; item: Connection }>('/me/tokens', { name: name.trim() });
			fresh = { id: r.item.id, token: r.token, name: r.item.name };
			items = [r.item, ...items];
			adding = false;
			name = '';
			copied = false;
		} catch {
			error = m['account.ai_failed']();
		} finally {
			working = false;
		}
	}

	async function renew(id: string) {
		working = true;
		error = null;
		try {
			const r = await api.post<{ item: Connection }>(`/me/tokens/${id}/renew`, {});
			items = items.map((c) => (c.id === id ? r.item : c));
			renewed = r.item.expires_at ? m['account.ai_renewed']({ date: when(r.item.expires_at) }) : null;
		} catch {
			error = m['account.ai_failed']();
		} finally {
			working = false;
		}
	}

	async function revoke(id: string) {
		working = true;
		error = null;
		try {
			await api.delete(`/me/tokens/${id}`);
			items = items.filter((c) => c.id !== id);
			if (fresh?.id === id) fresh = null;
			confirming = null;
		} catch {
			error = m['account.ai_failed']();
		} finally {
			working = false;
		}
	}

	async function copy(text: string) {
		try {
			await navigator.clipboard.writeText(text);
			copied = true;
		} catch {
			/* clipboard refused: the field is selectable */
		}
	}

	onMount(load);
</script>

{#if member}
	<section class="ai" data-testid="ai-connections">
		<h2>{m['account.ai_title']()}</h2>
		<p class="hint">{m['account.ai_hint']()}</p>

		{#if items.length}
			<ul class="list">
				{#each items as c (c.id)}
					<li>
						<span class="what">{c.name}<small>{m['account.ai_created']({ date: when(c.created_at) })} · {c.last_used_at ? m['account.ai_last_used']({ date: when(c.last_used_at) }) : m['account.ai_never_used']()}{#if c.expires_at}{' · '}<span class:ended={c.expired}>{c.expired ? m['account.ai_expired']({ date: when(c.expires_at) }) : m['account.ai_expires']({ date: when(c.expires_at) })}</span>{/if}</small></span>
						{#if confirming === c.id}
							<span class="confirm">
								<small>{m['account.ai_revoke_confirm']()}</small>
								<Button size="sm" variant="clear" onclick={() => (confirming = null)}>{m['account.ai_cancel']()}</Button>
								<Button size="sm" onclick={() => revoke(c.id)} disabled={working} data-testid="ai-revoke-yes">{m['account.ai_revoke_yes']()}</Button>
							</span>
						{:else}
							<span class="actions">
								<Button size="sm" variant="clear" onclick={() => renew(c.id)} disabled={working} data-testid="ai-renew">{m['account.ai_renew']()}</Button>
								<Button size="sm" variant="clear" onclick={() => (confirming = c.id)} data-testid="ai-revoke">{m['account.ai_revoke']()}</Button>
							</span>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}

		{#if renewed}<p class="hint" role="status">{renewed}</p>{/if}

		{#if fresh}
			<div class="fresh" data-testid="ai-fresh">
				<div class="row">
					<span class="label">{m['account.ai_new_token']()}</span>
					<button type="button" class="round" title={m['account.ai_close']()} aria-label={m['account.ai_close']()} onclick={() => (fresh = null)}><Icon name="x" /></button>
				</div>
				<div class="secret">
					<input id="ai-token" readonly value={fresh.token} onfocus={(e) => e.currentTarget.select()} />
					<Button size="sm" onclick={() => fresh && copy(fresh.token)}>{copied ? m['account.ai_copied']() : m['account.ai_copy']()}</Button>
				</div>
				<p class="hint">{m['account.ai_copy_hint']()} {m['account.ai_lifetime']()}</p>
				<div class="seg" role="group" aria-label={m['account.ai_app']()}>
					<button type="button" aria-pressed={app === 'desktop'} onclick={() => (app = 'desktop')}>{m['account.ai_desktop']()}</button>
					<button type="button" aria-pressed={app === 'code'} onclick={() => (app = 'code')}>{m['account.ai_code']()}</button>
				</div>
				{#if app === 'desktop'}
					<ol class="steps">
						<li><a href="/ai/scutoons.mcpb" download>{m['account.ai_desktop_1']()}</a></li>
						<li>{m['account.ai_desktop_2']()}</li>
						<li>{m['account.ai_desktop_3']()}</li>
					</ol>
				{:else}
					<ol class="steps">
						<li><a href="/ai/scutoons-mcp.mjs" download>{m['account.ai_code_1']()}</a></li>
					</ol>
					<pre class="command">{command}</pre>
					<p class="hint">{m['account.ai_code_2']()}</p>
				{/if}
			</div>
		{/if}

		{#if adding}
			<form class="add" onsubmit={create}>
				<label for="ai-name">{m['account.ai_name']()}</label>
				<input id="ai-name" bind:value={name} maxlength="100" placeholder={m['account.ai_name_placeholder']()} />
				<div class="row end">
					<Button size="sm" variant="clear" onclick={() => ((adding = false), (name = ''))}>{m['account.ai_cancel']()}</Button>
					<Button size="sm" type="submit" disabled={working || !name.trim()} data-testid="ai-create">{m['account.ai_create']()}</Button>
				</div>
			</form>
		{:else if !fresh}
			<Button size="sm" variant="secondary" onclick={() => (adding = true)} data-testid="ai-new">{m['account.ai_new']()}</Button>
		{/if}
		{#if error}<p class="error" role="alert">{error}</p>{/if}
	</section>
{/if}

<style>
	/* the account page's section look (its styles are scoped to the page) */
	section {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1rem;
	}
	h2 {
		font-size: 1rem;
		font-weight: 700;
		margin: 0;
	}
	.hint {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-muted);
	}
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
		gap: 0.25rem;
	}
	.ended {
		color: var(--state-error);
	}
	.confirm {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
	}
	.fresh {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.row.end {
		justify-content: flex-end;
	}
	.label {
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.secret {
		display: flex;
		gap: 0.5rem;
		min-width: 0;
	}
	.secret input {
		flex: 1;
		min-width: 0;
		min-height: var(--control-height);
		padding: 0 0.5rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--surface-tint-cyan);
		font-family: var(--font-mono);
		font-size: 0.875rem;
	}
	.steps {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin: 0;
		padding-left: 1.25rem;
	}
	.steps a {
		color: var(--ink);
	}
	.command {
		margin: 0;
		padding: 0.5rem 0.75rem;
		overflow-x: auto;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--surface-panel);
		font-size: 0.75rem;
		white-space: pre;
	}
	.add {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.add label {
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.add input {
		min-height: var(--control-height);
		padding: 0 0.5rem;
		border: var(--border-width-regular) solid var(--border-muted);
		font: inherit;
	}
	.error {
		margin: 0;
		color: var(--state-error);
	}
	/* the viewer has no tools.css: the segmented switch (grouped = square) and the round close button, locally */
	.seg {
		display: inline-flex;
		flex-wrap: wrap;
		align-self: flex-start;
	}
	.seg button {
		min-height: var(--control-height);
		padding: 0 0.75rem;
		margin-left: calc(-1 * var(--border-width-regular));
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		cursor: pointer;
	}
	.seg button:first-child {
		margin-left: 0;
	}
	.seg button[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	.round {
		display: inline-grid;
		place-items: center;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-full);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
	}
	.seg button:focus-visible,
	.round:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
</style>
