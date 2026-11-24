<script lang="ts">
	import type { Snippet } from 'svelte';
	import WritingHead from '$lib/components/writing/writing-head.svelte';

	/**
	 * The writing page (lap 2 C14): the text stands on the stage in one reading column of about 70 characters, large and
	 * open – no sheet, no box, no grey. Kanon writes in it, Notifications and Credits are lists of short texts in it, the
	 * Guide and an address Edit cannot open read in it. `lists` sets the column's text a step smaller (rows of short texts, not prose).
	 */
	let { title, sub = '', lists = false, testid = 'writing-page', children }: { title: string; sub?: string; lists?: boolean; testid?: string; children: Snippet } = $props();
</script>

<article class="writing-page" class:lists data-testid={testid}>
	<WritingHead {title} {sub} />
	{@render children()}
</article>

<style>
	/* the column: ~70 characters of the reading size, in the stage's middle; the panels float beside it */
	.writing-page {
		--wr-size: 1.18rem;
		box-sizing: content-box;
		max-width: 35em;
		margin: 0 auto;
		padding: 2.5rem 1.5rem 6rem;
		font-size: var(--wr-size);
		line-height: 1.62;
	}
	.writing-page.lists {
		--wr-size: 1rem;
		line-height: 1.5;
	}
</style>
