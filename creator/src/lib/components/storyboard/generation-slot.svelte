<script lang="ts">
	// One generation's still or video: from the member's workspace folder when it is there, else from the shared work
	// storage (through the API – members only), else a frame that says where the file is ("on <machine>").
	// storyboard-generations.ts placeOf.
	import { api } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { placeOf, workFile, type Generation } from '$lib/editor/storyboard-generations.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb, g }: { sb: StoryboardPage; g: Generation } = $props();

	const place = $derived(placeOf(g, sb.localFiles));
	let src = $state<string | null>(null);

	$effect(() => {
		const at = place;
		let url: string | null = null;
		let gone = false;
		const load = at.at === 'local' ? sb.readLocal(g.file!) : at.at === 'work' ? workFile(api, at.url) : Promise.resolve(null);
		load
			.then((blob) => {
				if (gone || !blob) return;
				url = URL.createObjectURL(blob);
				src = url;
			})
			.catch(() => {
				if (!gone) src = null;
			});
		return () => {
			gone = true;
			src = null;
			if (url) URL.revokeObjectURL(url);
		};
	});
	const where = $derived(place.at === 'elsewhere' ? m['storyboard.gen_elsewhere']({ machine: place.machine ?? m['storyboard.gen_other_machine']() }) : '');
</script>

<div class="slot" class:empty={!src} data-testid="generation-slot-{g.id}">
	{#if src && g.kind === 'video'}
		<!-- svelte-ignore a11y_media_has_caption -->
		<video {src} controls muted preload="metadata"></video>
	{:else if src}
		<img {src} alt={g.file ?? ''} />
	{:else if where}
		<span>{where}</span>
	{/if}
</div>

<style>
	.slot {
		display: grid;
		place-items: center;
		min-height: 6rem;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		box-sizing: border-box;
		overflow: hidden;
	}
	.slot.empty {
		border-style: dashed;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		padding: var(--space-05);
		text-align: center;
	}
	img,
	video {
		display: block;
		width: 100%;
		height: auto;
		max-height: 24rem;
		object-fit: contain;
	}
</style>
