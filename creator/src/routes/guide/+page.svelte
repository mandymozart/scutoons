<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import StyleGuide from '$lib/components/style-guide.svelte';
</script>


<div class="creator-page">
	<main class="guide">
		<h1>SCUTOONS {m['creator.style_guide']()}</h1>
		<StyleGuide />
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
</style>
