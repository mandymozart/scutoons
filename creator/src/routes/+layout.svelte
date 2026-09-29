<script lang="ts">
	import '@scutoons/tokens';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { AppShell, Button } from '@scutoons/ui';
	import { setCreatorSession } from '$lib/session.svelte.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LoginForm from '$lib/components/login-form.svelte';

	let { children }: { children: Snippet } = $props();

	const session = setCreatorSession();

	onMount(async () => {
		// magic link from the login mail: `<creator origin>/?login=<token>`
		const magicToken = page.url.searchParams.get('login');
		if (magicToken) {
			const url = new URL(page.url);
			url.searchParams.delete('login');
			goto(url.pathname + url.search, { replaceState: true });
			if (!session.token) {
				try {
					await session.verifyToken(magicToken);
				} catch (e) {
					console.warn('Magic link login failed', e);
				}
			}
		}
		await session.init();
	});

	// role gate: `creator` (the server checks it again on every content write)
	const waiting = $derived(!session.ready || (session.loading && !session.me));
	const isCreator = $derived(session.hasRole('creator'));
</script>

<svelte:head>
	<title>{m['creator.title']()}</title>
</svelte:head>

<div class="creator">
	{#if waiting}
		<p class="creator-gate">{m['creator.loading']()}</p>
	{:else if !session.isLoggedIn}
		<section class="creator-gate" data-testid="creator-sign-in">
			<img class="logo" src="/SCUTOONS-logo.svg" alt="SCUTOONS" width="200" />
			<h1>{m['creator.sign_in_title']()}</h1>
			<LoginForm />
		</section>
	{:else if !isCreator}
		<section class="creator-gate" data-testid="creator-no-access">
			<h1>{m['creator.no_access_title']()}</h1>
			<p>{m['creator.no_access_text']({ email: session.user?.email ?? '' })}</p>
			<p class="gate-actions">
				<Button href={VIEWER_URL}>{m['creator.open_reader']()}</Button>
				<Button variant="secondary" onclick={() => session.logout()}>{m['creator.sign_out']()}</Button>
			</p>
		</section>
	{:else}
		<!-- Pages bring their own top bar (and the editor its own columns); the shell reserves the bottom ActionBar,
		     so a page never scrolls by a stray amount and side borders end exactly at the bar -->
		<AppShell>
			{@render children()}
		</AppShell>
	{/if}
</div>

<style>
	/* Editor chrome: design system Part 1 – IBM Plex Mono, ink/paper, square, 2px structural borders.
	   Dense desktop tool: fixed px base instead of the reader's fluid root size. */
	.creator {
		/* editor aliases of @scutoons/tokens (ink, paper, border-*, surface-* come from the tokens directly) */
		--cyan: var(--primary-cyan);
		--muted: var(--text-muted);
		--error: var(--state-error);
		--warn: var(--state-warning);
		/* every creator page: the bottom ActionBar mirrors the 64px top bars (AppShell reserves this much) */
		--action-bar-height: 64px;
		font-family: var(--font-mono);
		font-size: 13px;
		line-height: 1.4;
		color: var(--ink);
		background: var(--paper);
		min-height: 100dvh;
	}

	.creator-gate {
		max-width: 32rem;
		margin: 0 auto;
		padding: 6rem 1rem 2rem;
		font-size: 16px;
	}
	.creator-gate h1 {
		font-size: 1.25rem;
	}
	.creator-gate .logo {
		display: block;
		margin-bottom: var(--space-1);
	}
	.gate-actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}

	/* Dense editor controls (@scutoons/ui components style themselves) */
	:global(:where(.creator) input:not([type='checkbox']):not([type='radio']):not([type='range']):not([type='color'])),
	:global(:where(.creator) select),
	:global(:where(.creator) textarea) {
		width: 100%;
		margin: 0;
		padding: 0.3em 0.45em;
		font: inherit;
		font-size: 12px;
		border: 1px solid var(--border-subtle);
		border-radius: 0;
		background: var(--paper);
		box-sizing: border-box;
	}
	:global(:where(.creator) input:focus),
	:global(:where(.creator) select:focus),
	:global(:where(.creator) textarea:focus) {
		outline: 2px solid var(--ink);
		outline-offset: -1px;
	}
	:global(:where(.creator) input[type='checkbox']),
	:global(:where(.creator) input[type='radio']) {
		width: auto;
		margin: 0;
	}
	:global(:where(.creator) button),
	:global(:where(.creator) .btn) {
		font: inherit;
		font-size: 12px;
		padding: 0.45em 1em;
		border-radius: 0;
		line-height: 1.2;
		background: var(--ink);
		color: var(--paper);
		border: 2px solid var(--ink);
		cursor: pointer;
		text-decoration: none;
		display: inline-block;
	}
	:global(:where(.creator) button.secondary),
	:global(:where(.creator) .btn.secondary) {
		background: var(--paper);
		color: var(--ink);
		border-color: var(--border-muted);
	}
	:global(:where(.creator) button.secondary:hover:not(:disabled)) {
		background: var(--paper);
		border-color: var(--ink);
	}
	:global(:where(.creator) button.ghost) {
		background: transparent;
		color: var(--ink);
		border-color: transparent;
		padding: 0.2em 0.4em;
	}
	:global(:where(.creator) button.ghost:hover:not(:disabled)) {
		background: var(--surface-hover);
	}
	:global(:where(.creator) button.active),
	:global(:where(.creator) button.active:hover:not(:disabled)) {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	:global(:where(.creator) button:disabled) {
		opacity: 0.4;
		cursor: not-allowed;
	}
	:global(:where(.creator) h1),
	:global(:where(.creator) h2),
	:global(:where(.creator) h3) {
		font-weight: 700;
		margin: 0 0 0.5em;
	}
	:global(:where(.creator) label) {
		display: block;
	}
	:global(:where(.creator) .hint) {
		color: var(--muted);
		font-size: 11px;
		line-height: 1.35;
	}
</style>
