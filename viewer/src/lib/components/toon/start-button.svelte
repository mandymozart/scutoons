<script lang="ts">
	import { isItemPublic, readingState, type LoadedSequenceItem } from '@scutoons/content';
	import { getGameContext } from '$lib/contexts/game.svelte.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The season's one start button (agents/DESIGN.md "Start button"): "S1 K1 lesen" / "S1 K4 weiterlesen" (+ how far,
	 * as a thin bar) / the next chapter / "S1 K1 erneut lesen", from the reading progress (game context checkpoints:
	 * localStorage, a member's `/api/progress/<toon>`). Only readable chapters are targets, never "coming soon".
	 */
	let { toonId, season, chapters }: { toonId: string; season: number; chapters: LoadedSequenceItem[] } = $props();

	const game = getGameContext();
	const readable = $derived(chapters.filter((c) => isItemPublic(c, import.meta.env.DEV)));
	const reading = $derived(readingState(toonId, readable.map((c) => c.number), game.state.progress));
	const target = $derived(reading ? readable.find((c) => c.number === reading.chapter) : undefined);
	const label = $derived.by(() => {
		if (!reading) return '';
		const values = { season, chapter: reading.chapter };
		if (reading.kind === 'continue') return m['start.continue'](values);
		if (reading.kind === 'again') return m['start.again'](values);
		return m['start.read'](values);
	});
	// the bar only for a chapter in progress, from the stored position (slices seen / the chapter's slices)
	const percent = $derived(reading?.kind === 'continue' ? Math.max(1, Math.round(reading.position * 100)) : null);
</script>

{#if reading && target}
	<div class="start">
		<a
			class="start-button"
			href={localizeHref(`/${toonId}/chapter/${reading.chapter}`)}
			title={m['start.title']({ season, chapter: reading.chapter, title: target.title ?? '' })}
			data-testid="start-button"
			data-state={reading.kind}
		>
			{label}
			<svg class="start-arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
				<path d="M1 8h13M9 3l5 5-5 5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="square" />
			</svg>
		</a>
		{#if percent !== null}
			<!-- the reading progress under the button, as wide as the button -->
			<span class="start-progress" aria-hidden="true" data-testid="start-progress"><span style:width="{percent}%"></span></span>
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
		gap: var(--space-075);
		padding: var(--space-1) var(--space-15);
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
	.start-arrow {
		flex: none;
		width: 1em;
		height: 1em;
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
</style>
