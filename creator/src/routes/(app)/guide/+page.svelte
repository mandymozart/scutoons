<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import StyleGuide from '$lib/components/page/style-guide.svelte';
	import ScriptRules from '$lib/components/script/script-rules.svelte';
	import UiKit from '$lib/components/page/ui-kit.svelte';
	import WritingPage from '$lib/components/writing/writing-page.svelte';
	import { onMount, tick } from 'svelte';

	// a link with an anchor (e.g. /guide#script-format from the script page): the page renders after sign-in, too late for
	// the browser's own jump – scroll there once it is rendered
	onMount(async () => {
		await tick();
		const id = decodeURIComponent(location.hash.slice(1));
		if (id) document.getElementById(id)?.scrollIntoView({ block: 'start' });
	});
</script>

<!-- the guide reads in the writing page's column (lap 2 C14); the UI kit below it is wider – its components side by side -->
<WritingPage title={m['creator.style_guide']()} sub="SCUTOONS" testid="guide">
	<div class="guide">
		<StyleGuide />
		<section id="script-format" class="script-format">
			<h2>{m['creator.script_rules']()}</h2>
			<ScriptRules />
		</section>
	</div>
</WritingPage>
<!-- the UI kit (RULES H12): every component by night and by day, to hold against the deck editor -->
<section id="ui-kit" class="ui-kit">
	<h2>UI kit</h2>
	<UiKit />
</section>

<style>
	/* the guide's own headings as the writing page's (no capitals, no bold) */
	.guide :global(h3),
	h2 {
		margin: 1.6em 0 0.4em;
		font-size: 1.15em;
		font-weight: normal;
		text-transform: none;
		letter-spacing: normal;
	}
	.guide :global(h3:first-child) {
		margin-top: 0;
	}
	.guide :global(.style-guide),
	.guide :global(.rules) {
		line-height: inherit;
	}
	.script-format {
		scroll-margin-top: 4rem;
	}
	.ui-kit {
		max-width: 72rem;
		margin: 0 auto;
		padding: 0 1.5rem 3rem;
		scroll-margin-top: var(--shell-top);
	}
</style>
