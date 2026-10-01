<script lang="ts">
	import { getLocale, locales, setLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { WWW_URL } from '$lib/config.ts';
	import { Button } from '@scutoons/ui';
	import { createAuthSession, SignedInAs } from '@scutoons/auth';
	import { onMount } from 'svelte';
	import { authApi } from '$lib/api.ts';

	const locale = getLocale();

	// single sign-on: an account signed in on any SCUTOONS app shows here (the shop itself needs no account)
	const session = createAuthSession({ api: authApi, locale: getLocale });
	onMount(() => void session.init());
</script>

<!-- Same place and size as the viewer's logo (main menu toggle); here it leads back to www -->
<header class="shop-nav">
	<a class="home" href={`${WWW_URL}/`} aria-label={m['nav.home']()} title={m['nav.home']()}>
		<img src="/SCUTOONS-logo-animated.svg" alt="SCUTOONS" />
	</a>
	<div class="corner">
		<SignedInAs {session} {locale} compact />
		<nav class="language" aria-label={m['nav.language']()}>
			{#each locales as l (l)}
				<Button
					variant={l === locale ? 'primary' : 'secondary'}
					size="sm"
					aria-pressed={l === locale}
					onclick={() => setLocale(l)}>{l}</Button
				>
			{/each}
		</nav>
	</div>
</header>

<style>
	.home {
		position: fixed;
		top: 0.5rem;
		left: 0;
		z-index: 10;
		display: block;
		touch-action: manipulation;
	}

	.home img {
		height: 3rem;
	}

	.corner {
		position: fixed;
		top: 0.5rem;
		right: 0.5rem;
		z-index: 10;
		display: flex;
		align-items: center;
		gap: var(--space-05);
		max-width: calc(100vw - 6rem);
	}
	.corner :global(.sc-signed-in) {
		min-width: 0;
		padding: 0 var(--space-05);
		background: var(--paper);
	}

	.language {
		display: flex;
		flex: none;
		gap: var(--space-025);
	}
</style>
