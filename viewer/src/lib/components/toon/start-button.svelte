<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { readingState, type LoadedSequenceItem } from '@scutoons/content';
	import { getGameContext } from '$lib/contexts/game.svelte.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { isReadable } from '$lib/draft-preview.svelte.ts';
	import { chapterShownNumber } from '$lib/utils/chapter-name.ts';

	/**
	 * The season's one start button (agents/DESIGN.md "Start button"): "S1 K1 lesen" / "S1 K4 weiterlesen" (+ how far,
	 * as a thin bar) / the next chapter / "S1 K1 erneut lesen" – and "Prolog ansehen" (chapter 0) for a visitor who
	 * read nothing yet and didn't watch the prologue to its end – from the reading progress (game context checkpoints:
	 * localStorage, a member's `/api/progress/<toon>`). Only readable chapters are targets, never "coming soon".
	 */
	let {
		toonId,
		season,
		chapters,
		inverse = false,
		testid = 'start-button'
	}: {
		toonId: string;
		season: number;
		chapters: LoadedSequenceItem[];
		/** on a dark background (the desktop hero): paper button, ink text */
		inverse?: boolean;
		testid?: string;
	} = $props();

	const game = getGameContext();
	const readable = $derived(chapters.filter((c) => isReadable(c)));
	const reading = $derived(readingState(toonId, readable.map((c) => c.number), game.state.progress));
	const target = $derived(reading ? readable.find((c) => c.number === reading.chapter) : undefined);
	const label = $derived.by(() => {
		if (!reading) return '';
		// K = the position in the season (BACKLOG 10.41), never the absolute number
		const values = { season, chapter: chapterShownNumber(reading.chapter) ?? reading.chapter };
		if (reading.kind === 'prologue') return m['start.prologue']();
		if (reading.kind === 'continue') return m['start.continue'](values);
		if (reading.kind === 'again') return m['start.again'](values);
		return m['start.read'](values);
	});
	// the bar only for a chapter in progress, from the stored position (slices seen / the chapter's slices)
	const percent = $derived(reading?.kind === 'continue' ? Math.max(1, Math.round(reading.position * 100)) : null);
	// "continue" opens the chapter where the reader stopped (`?resume`, the chapter page scrolls there – BACKLOG 10.69)
	const href = $derived(reading ? `${localizeHref(`/${toonId}/chapter/${reading.chapter}`)}${reading.kind === 'continue' && reading.spot ? '?resume' : ''}` : '');
</script>

{#if reading && target}
	<div class="start" class:inverse>
		<a
			class="start-button sc-control"
			{href}
			title={reading.kind === 'prologue'
				? m['start.prologue_title']({ season, title: target.title ?? '' })
				: m['start.title']({ season, chapter: chapterShownNumber(reading.chapter) ?? reading.chapter, title: target.title ?? '' })}
			data-testid={testid}
			data-state={reading.kind}
		>
			{label}
			<!-- forward = the chevron after the label (DESIGN.md "Direction arrows") -->
			<Icon name="chevron-right" size="1rem" />
		</a>
		{#if percent !== null}
			<!-- the reading progress under the button, as wide as the button -->
			<span class="start-progress" aria-hidden="true" data-testid={`${testid === 'start-button' ? 'start' : testid}-progress`}><span style:width="{percent}%"></span></span>
		{/if}
	</div>
{/if}

<style>
	/* centred, as wide as the button; the progress rule sits under it */
	.start {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		width: fit-content;
		margin-inline: auto;
	}
	/* the primary button (ink / paper, square), as a link */
	.start-button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.25rem;
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 var(--space-15);
		background: var(--ink);
		color: var(--paper);
		font-family: var(--font-mono);
		font-size: var(--type-button-size);
		line-height: var(--type-button-line);
		white-space: nowrap;
		text-decoration: none;
		border-radius: var(--radius-none);
		touch-action: manipulation;
	}
	/* a 2px rule: the part read in ink, the rest of the chapter in border-muted */
	.start-progress {
		display: block;
		height: var(--border-width-regular);
		background: var(--border-muted);
	}
	.start-progress span {
		display: block;
		height: 100%;
		background: var(--ink);
	}
	/* inverted for the dark hero: paper button, the read part of the rule in paper */
	.inverse .start-button {
		background: var(--paper);
		color: var(--ink);
		font-size: inherit;
		padding: 0 1.6em;
	}
	.inverse .start-progress {
		background: rgb(255 255 255 / 0.35);
	}
	.inverse .start-progress span {
		background: var(--paper);
	}
</style>
