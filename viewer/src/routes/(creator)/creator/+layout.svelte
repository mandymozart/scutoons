<script lang="ts">
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import LoginForm from '$lib/components/account/login-form.svelte';

	let { children }: { children: Snippet } = $props();

	const session = getSessionContext();

	// the root layout loads the session on mount; until then a stored token means "wait"
	let mounted = $state(false);
	onMount(() => (mounted = true));

	const waiting = $derived(!mounted || (!!session.token && !session.ready) || (session.loading && !session.me));
	const isCreator = $derived(!!session.user?.roles?.includes('creator'));
</script>

<svelte:head>
	<title>{m['creator.title']()}</title>
</svelte:head>

<div class="creator">
	{#if waiting}
		<p class="creator-gate">{m['creator.loading']()}</p>
	{:else if !session.isLoggedIn}
		<section class="creator-gate">
			<h1>{m['creator.sign_in_title']()}</h1>
			<p>{m['creator.sign_in_text']()}</p>
			<LoginForm />
		</section>
	{:else if !isCreator}
		<section class="creator-gate" data-testid="creator-no-access">
			<h1>{m['creator.no_access_title']()}</h1>
			<p>{m['creator.no_access_text']({ email: session.user?.email ?? '' })}</p>
			<p><a class="btn" href={localizeHref('/')}>{m['creator.back_to_reader']()}</a></p>
		</section>
	{:else}
		{@render children()}
	{/if}
</div>

<style>
	/* Editor chrome: design system Part 1 – IBM Plex Mono, ink/paper, square, 2px structural borders.
	   Dense desktop tool: fixed px base instead of the reader's fluid root size. */
	.creator {
		--ink: #000;
		--paper: #fff;
		--cyan: #00d1ff;
		--muted: #6b6b6b;
		--border-muted: #ccc;
		--border-subtle: #ddd;
		--surface-hover: #f0f0f0;
		--surface-panel: #f5f5f5;
		--error: #ff210a;
		--warn: #b07a00;
		font-family: 'IBM Plex Mono', monospace;
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

	/* Local reset of the reader's big form controls (styles.css) */
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
