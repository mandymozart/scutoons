<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import StyleGuide from '$lib/components/page/style-guide.svelte';
	import ScriptRules from '$lib/components/script/script-rules.svelte';
	import { onMount, tick } from 'svelte';

	// a link with an anchor (e.g. /guide#script-format from the script page): the page renders after sign-in, too late for
	// the browser's own jump – scroll there once it is rendered
	onMount(async () => {
		await tick();
		const id = decodeURIComponent(location.hash.slice(1));
		if (id) document.getElementById(id)?.scrollIntoView({ block: 'start' });
	});
</script>


<div class="creator-page">
	<main class="guide">
		<h1>SCUTOONS {m['creator.style_guide']()}</h1>
		<StyleGuide />
		<!-- the screenplay format (moved here from the script page's side column, owner 2026-10-03) -->
		<section id="script-format" class="script-format">
			<h2>{m['creator.script_rules']()}</h2>
			<ScriptRules />
		</section>
	</main>
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
	.script-format {
		margin-top: 2rem;
		padding-top: 1rem;
		border-top: var(--border-width-regular) solid var(--ink);
		scroll-margin-top: 4rem;
	}
</style>
