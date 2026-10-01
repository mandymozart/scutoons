<script lang="ts">
	import { ActionBar } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { infoBar } from '$lib/info.ts';
	import StyleGuide from '$lib/components/style-guide.svelte';
	import PageHelp from '$lib/components/page-help.svelte';
	import UpLink from '$lib/components/up-link.svelte';
</script>


<div class="creator-page">
	<main class="guide">
		<h1>SCUTOONS {m['creator.style_guide']()}</h1>
		<StyleGuide />
	</main>
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()} {...infoBar()} infoHref={undefined} infoTitle={m['creator.style_guide']()}>
		{#snippet info()}<PageHelp page="guide" />{/snippet}
		{#snippet start()}
			<UpLink />
		{/snippet}
	</ActionBar>
</div>

<style>
	.guide {
		max-width: 44rem;
		padding: 1rem 1rem 3rem;
	}
	.guide h1 {
		font-size: var(--type-title-size);
		line-height: var(--type-title-line);
	}
</style>
