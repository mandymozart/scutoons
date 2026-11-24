<script lang="ts">
	import { lineInfo, pieces } from '$lib/editor/writing.ts';

	/** A paragraph shown as the writing page shows it, read-only (the source in Compare): its lines' kinds, its marks left out. */
	let { text, lang }: { text: string; lang: string } = $props();
	const lines = $derived(text.split('\n').map((line) => ({ kind: lineInfo(line).kind, pieces: pieces(line) })));
</script>

<div class="read" {lang}>
	{#each lines as line, i (i)}
		<p class="wr-line wr-{line.kind}">{#each line.pieces as piece, k (k)}{#if piece.strong}<strong class="sc-emphasis">{piece.text}</strong>{:else if piece.em}<em>{piece.text}</em>{:else if piece.link}<u>{piece.text}</u>{:else}{piece.text}{/if}{/each}{#if !line.pieces.length}&nbsp;{/if}</p>
	{/each}
</div>

<style>
	p {
		margin: 0;
	}
	u {
		text-underline-offset: 0.2em;
	}
	.wr-heading {
		font-size: var(--wr-heading, 1.15em);
		line-height: 1.3;
		padding: 0.6em 0 0.15em;
	}
	.wr-speaker {
		font-size: var(--wr-label, 0.62em);
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--tool-muted);
		padding-top: 1.4em;
	}
	.wr-direction {
		color: var(--tool-muted);
		font-style: italic;
	}
	.wr-quote {
		font-style: italic;
	}
</style>
