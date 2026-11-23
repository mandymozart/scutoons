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
	/* the docs in the tools' colours (RULES H11): the stage, its text, lines (night; day where the page says data-theme="day") */
	.docs {
		--d-bg: var(--tool-ground);
		--d-ink: var(--tool-text);
		--d-muted: var(--tool-muted);
		--d-line: var(--tool-line);
		--d-card: transparent;
		--d-hover: var(--tool-hover);
		--d-accent: var(--tool-text);
		--d-logo: invert(1);
		min-height: 100vh;
		background: var(--d-bg);
		color: var(--d-ink);
		font-family: var(--font-mono);
	}
	:global(html[data-theme='day']) .docs {
		--d-logo: none;
	}
	:global(body):has(.docs) {
		background: var(--tool-ground);
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
