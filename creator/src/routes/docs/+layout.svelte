<script lang="ts">
	// The docs' frame: the logo, the way into the creator, the page. No sign-in, no paraglide (English for now).
	// Night is the default look; a system set to light gets the day variant.
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
</script>

<svelte:head>
	<meta name="theme-color" content="#0b0b0b" />
</svelte:head>

<div class="docs">
	<header class="top">
		<a class="home" href="/docs/"><img src="/SCUTOONS-logo.svg" alt="SCUTOONS" /><span>Creator docs</span></a>
		<a class="open" href="/">Open the creator</a>
	</header>
	<main>{@render children()}</main>
</div>

<style>
	.docs {
		--d-bg: #0b0b0b;
		--d-ink: #f2f2f2;
		--d-muted: #a0a0a0;
		--d-line: #2c2c2c;
		--d-card: #151515;
		--d-hover: #1f1f1f;
		--d-accent: var(--primary-cyan);
		--d-logo: invert(1);
		min-height: 100vh;
		background: var(--d-bg);
		color: var(--d-ink);
		font-family: var(--font-mono);
	}
	@media (prefers-color-scheme: light) {
		.docs {
			--d-bg: var(--paper);
			--d-ink: var(--ink);
			--d-muted: var(--text-muted);
			--d-line: var(--border-subtle);
			--d-card: var(--paper);
			--d-hover: var(--surface-hover);
			--d-logo: none;
		}
	}
	:global(body):has(.docs) {
		background: var(--d-bg, #0b0b0b);
	}
	.top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.5rem 1rem;
		border-bottom: 2px solid var(--d-line);
	}
	.home {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		color: inherit;
		text-decoration: none;
	}
	.home img {
		width: 2.5rem;
		height: 2.5rem;
		filter: var(--d-logo);
	}
	.open {
		display: inline-flex;
		align-items: center;
		min-height: var(--control-height);
		padding: 0 0.75rem;
		border: 2px solid var(--d-ink);
		color: inherit;
		text-decoration: none;
	}
	.open:hover {
		background: var(--d-hover);
	}
	main {
		max-width: 60rem;
		margin: 0 auto;
		padding: 2rem 1rem 4rem;
	}
	main :global(a) {
		color: inherit;
	}
	main :global(:focus-visible) {
		outline: 2px solid var(--d-accent);
		outline-offset: 2px;
	}
</style>
