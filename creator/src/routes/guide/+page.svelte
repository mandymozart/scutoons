<script lang="ts">
	import { ActionBar } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { infoBar } from '$lib/info.ts';
	import StyleGuide from '$lib/components/style-guide.svelte';
	import PageHelp from '$lib/components/page-help.svelte';
</script>

<svelte:head>
	<title>{m['creator.style_guide']()} · {m['creator.title']()}</title>
</svelte:head>

<div class="creator-page">
	<header class="creator-top">
		<nav class="crumbs"><a href="/">{m['creator.title']()}</a> / <strong>{m['creator.style_guide']()}</strong></nav>
	</header>
	<main class="guide">
		<h1>SCUTOONS {m['creator.style_guide']()}</h1>
		<StyleGuide />
	</main>
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()} {...infoBar()} infoHref={undefined} infoTitle={m['creator.style_guide']()}>
		{#snippet info()}<PageHelp page="guide" />{/snippet}
		{#snippet start()}
			<a class="bar-link" href="/">← {m['creator.toons']()}</a>
		{/snippet}
	</ActionBar>
</div>

<style>
	.creator-top {
		display: flex;
		align-items: center;
		min-height: 64px;
		padding: 0 1rem 0 96px;
		border-bottom: 2px solid var(--ink);
	}
	.crumbs a,
	.bar-link {
		color: var(--ink);
	}
	.guide {
		max-width: 44rem;
		padding: 1rem 1rem 3rem;
	}
	.guide h1 {
		font-size: 16px;
	}
	.guide :global(.style-guide) {
		font-size: 13px;
	}
</style>
