<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import type { Marks } from '$lib/editor/slice-marks.ts';

	/**
	 * The chapter tree's marks of what happens on a slice / in a group (lib/editor/slice-marks.ts): text boxes, placed
	 * items, audio, video, map places, events – an icon each, with its count when there is more than one; nothing shown
	 * for what isn't there. The full words in the tooltip.
	 */
	let { marks, testid }: { marks: Marks; testid?: string } = $props();

	const shown = $derived(
		[
			{ key: 'texts', icon: 'type', label: m['creator.text_boxes'](), n: marks.texts },
			{ key: 'items', icon: 'gift', label: m['creator.marks_items'](), n: marks.items },
			{ key: 'audio', icon: 'music', label: m['creator.audio'](), n: marks.audio },
			{ key: 'video', icon: 'film', label: m['creator.marks_video'](), n: marks.video },
			{ key: 'places', icon: 'map-pin', label: m['creator.marks_places'](), n: marks.places },
			{ key: 'events', icon: 'zap', label: m['creator.events'](), n: marks.events }
		].filter((mark) => mark.n > 0)
	);
	const title = $derived(shown.map((mark) => `${mark.label}: ${mark.n}`).join(' · '));
</script>

{#if shown.length}
	<span class="marks" {title} data-testid={testid}>
		{#each shown as mark (mark.key)}
			<span class="mark" data-mark={mark.key}><Icon name={mark.icon} size="0.875rem" label={mark.label} />{#if mark.n > 1}{mark.n}{/if}</span>
		{/each}
	</span>
{/if}

<style>
	.marks {
		display: inline-flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.15rem 0.4rem;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.mark {
		display: inline-flex;
		align-items: center;
		gap: 0.1rem;
	}
</style>
