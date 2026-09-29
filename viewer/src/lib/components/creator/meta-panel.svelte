<script lang="ts">
	import type { ContentDoc } from '$lib/creator/doc.svelte.ts';
	import { Status } from '$lib/models/enums.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PropField from './prop-field.svelte';
	import LocaleText from './locale-text.svelte';
	import ImagePicker from './image-picker.svelte';

	/** Chapter meta. Title/description per locale; the rest shared, with per-locale overrides shown. */
	let { doc, images = [] }: { doc: ContentDoc; images?: string[] } = $props();

	const ch = $derived(doc.get(doc.primary, []));
	const stringKeyvisual = $derived(doc.present.filter((l) => typeof doc.get(l, ['keyvisual']) === 'string'));

	function convertKeyvisual() {
		doc.structural((file) => {
			if (typeof file.keyvisual === 'string') file.keyvisual = { image: file.keyvisual };
		});
	}
</script>

<section class="meta-panel" data-testid="meta-panel">
	<h2>{m['creator.meta']()} · chapter{ch?.number}.json</h2>

	<LocaleText {doc} path={['title']} label="title" multiline={false} />
	<LocaleText {doc} path={['description']} label="description" rows={3} />

	<div class="grid g3">
		<PropField {doc} path={['status']} label="status" type="select" options={Object.values(Status)} required hint="Also set the status in the season (toon page)." />
		<PropField {doc} path={['release']} label="release" type="date" required />
		<PropField {doc} path={['author']} label="author" required />
		<PropField {doc} path={['number']} label="number" type="number" step={1} required />
		<PropField {doc} path={['published']} label="published (flag)" type="bool" required />
	</div>

	<div class="grid g2">
		<ImagePicker toon={doc.toon} label="thumbnail" value={ch?.thumbnail} known={images} onchange={(v) => doc.setShared(['thumbnail'], v)} />
		<div>
			{#if stringKeyvisual.length}
				<p class="warn">
					keyvisual is a string in {stringKeyvisual.join(', ')} – the type expects <code>{'{ image, video? }'}</code>.
					<button type="button" class="secondary" onclick={convertKeyvisual}>Convert</button>
				</p>
			{:else}
				<ImagePicker toon={doc.toon} label="keyvisual.image" value={ch?.keyvisual?.image} known={images} onchange={(v) => doc.setShared(['keyvisual', 'image'], v)} />
				<PropField {doc} path={['keyvisual', 'video']} label="keyvisual.video" placeholder="videos/s1c2-key-visual.mp4" />
			{/if}
		</div>
	</div>
	{#each doc.present.filter((l) => l !== doc.primary) as l}
		{#if doc.get(l, ['thumbnail']) !== ch?.thumbnail}
			<p class="hint">{l} thumbnail: {doc.get(l, ['thumbnail'])} ({m['creator.override']()})
				<button type="button" class="ghost" onclick={() => doc.resetOverride(l, ['thumbnail'])}>{m['creator.use_shared']()}</button></p>
		{/if}
	{/each}
</section>

<style>
	.meta-panel {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-width: 56rem;
	}
	h2 {
		font-size: 14px;
	}
	.grid {
		display: grid;
		gap: 0.5rem 0.75rem;
	}
	.g3 {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}
	.g2 {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}
	.warn {
		color: var(--warn);
	}
</style>
