<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { statusWord } from './status-badges.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { chapterSyncChanges, seasonStatusOf, setSeasonStatus, STATUSES, syncChapter } from '$lib/editor/visibility.ts';
	import PropField from './prop-field.svelte';
	import LocaleText from './locale-text.svelte';
	import ImagePicker from './image-picker.svelte';

	/**
	 * Chapter meta. Title/description per locale; the rest shared, with per-locale overrides shown.
	 * `season`: season 1 – the chapter's status (visibility in the reader) lives there, per locale; this is the only place
	 * where it is changed (lists show it read-only). `featuring`: the chapter's featuring field (chapter editor).
	 */
	let {
		doc,
		season = null,
		number,
		images = [],
		featuring
	}: { doc: ContentDoc; season?: ContentDoc | null; number: number; images?: string[]; featuring?: Snippet } = $props();

	const ch = $derived(doc.get(doc.primary, []));
	const statusLocales = $derived(season && !season.loading ? season.locales : []);
	const syncChanges = $derived(chapterSyncChanges(doc, season, number));
	const stringKeyvisual = $derived(doc.present.filter((l) => typeof doc.get(l, ['keyvisual']) === 'string'));

	function convertKeyvisual() {
		doc.structural((file) => {
			if (typeof file.keyvisual === 'string') file.keyvisual = { image: file.keyvisual };
		});
	}
</script>

<section class="meta-panel" data-testid="meta-panel">
	<h2>{m['creator.meta']()}</h2>

	<LocaleText {doc} path={['title']} label="title" multiline={false} />
	<LocaleText {doc} path={['description']} label="description" rows={3} />

	{@render featuring?.()}

	<fieldset class="status" data-testid="chapter-status">
		<legend>{m['creator.status']()}</legend>
		{#if !season || season.loading}
			<p class="hint">{m['creator.loading']()}</p>
		{:else}
			<div class="status-row">
				{#each statusLocales as l (l)}
					{@const status = seasonStatusOf(season, l, number)}
					<label class="status-field">
						<span>{l}{l === season.primary ? ` (${m['creator.source']()})` : ''}</span>
						{#if status === undefined}
							<span class="hint">{m['creator.status_no_season']()}</span>
						{:else if status === null}
							<span class="error">{m['creator.status_not_in_season']()}</span>
						{:else}
							<select value={status} onchange={(e) => setSeasonStatus(season, l, number, e.currentTarget.value as (typeof STATUSES)[number])} data-testid="status-{l}">
								{#each STATUSES as s}<option value={s}>{statusWord(s)}</option>{/each}
							</select>
						{/if}
					</label>
				{/each}
			</div>
			{#if syncChanges.length}
				<p class="hint" data-testid="status-sync">
					{m['creator.status_sync']()} <span>({syncChanges.join(' · ')})</span>
					<button type="button" class="ghost" onclick={() => syncChapter(doc, season, number)}>{m['creator.status_sync_now']()}</button>
				</p>
			{/if}
		{/if}
	</fieldset>

	<div class="grid g3">
		<PropField {doc} path={['release']} label="release" type="date" required />
		<PropField {doc} path={['author']} label="author" required />
		<PropField {doc} path={['number']} label="number" type="number" step={1} required />
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
				<PropField {doc} path={['keyvisual', 'video']} label="keyvisual.video" placeholder="s1/c2/s1c2-key-visual.mp4" />
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
	.error {
		color: var(--error);
	}
	.status {
		border: 1px solid var(--border-muted);
		padding: 0.5rem 0.75rem;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		align-items: flex-start;
	}
	.status legend {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		padding: 0 0.3rem;
	}
	.status-row {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 1.25rem;
	}
	.status-field {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
	}
	.status-field select {
		width: auto;
	}
	.status .hint {
		margin: 0;
	}
</style>
