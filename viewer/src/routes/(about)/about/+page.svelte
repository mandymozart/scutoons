<script lang="ts">
	/**
	 * About SCUTOONS (owner 2026-10-31, #214): how SCUTOONS came to life – a neurodiversity in which music suggests
	 * stories and pictures – and the name's origin (Kiki Hitomi; SCU + TOON, heard as tunes). The owner's text, confirmed
	 * from agents/research/ABOUT.md; the text lives in the messages. Linked from the landing page's footer.
	 */
	import { m } from '$lib/paraglide/messages.js';
</script>

<svelte:head><title>{m.about()} · SCUTOONS</title></svelte:head>

<div class="page" data-testid="about">
	<h1>{m.about()}</h1>

	<section>
		<h2>{m['about_page.origin_title']()}</h2>
		<p>{m['about_page.origin_1']()}</p>
		<p>{m['about_page.origin_2']()}</p>
		<p>{m['about_page.origin_3']()}</p>
	</section>

	<section>
		<h2>{m['about_page.name_title']()}</h2>
		<p>{m['about_page.name_1']()}</p>
	</section>
</div>

<style>
	.page {
		padding: 2rem;
		max-width: 50rem;
		margin: 0 auto;
	}

	h1 {
		margin-bottom: 1rem;
	}

	section {
		margin: 2rem 0;
	}

	h2 {
		margin-bottom: 1rem;
	}

	p {
		margin: 0 0 1rem;
		line-height: 1.5;
	}
</style>
