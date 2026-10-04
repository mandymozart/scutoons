<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { statusWord } from '$lib/editor/status-word.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { chapterSyncChanges, seasonStatusOf, setSeasonStatus, STATUSES, syncChapter } from '$lib/editor/visibility.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import LocaleText from '$lib/components/fields/locale-text.svelte';
	import { MediaSlot } from '@scutoons/media';
	import { fileLabel } from '$lib/labels.ts';
	import { entities } from '@scutoons/content';

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
	/** the chapter's folder by the content rules – uploads from its fields land there (catpope/s1/c2/) */
	const place = $derived(`${doc.toon}/s${Number(season?.get(season.primary, ['number'])) || 1}/c${number}/`);
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
	<!-- the chapter's number, prominent (owner 2026-10-03): it is its identity – URL, release order – and never changes -->
	<header class="chapter-head" data-testid="chapter-number">
		<h2>{fileLabel(entities.chapter(doc.toon, number))}</h2>
		<p class="hint">{m['creator.chapter_number_hint']({ path: `/${doc.toon}/chapter/${number}` })}</p>
	</header>

	<LocaleText {doc} path={['title']} label="title" multiline={false} />
	<LocaleText {doc} path={['description']} label="description" rows={3} />

	<div class="status" data-testid="chapter-status">
		<span class="sc-label">{m['creator.status']()}</span>
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
	</div>

	<div class="sc-fields" data-cols="2">
		<PropField {doc} path={['release']} label="release" type="date" required />
		<PropField {doc} path={['author']} label="author" required />
	</div>

	<!-- the images as slots: the preview is the button, no path fields (owner 2026-10-03) -->
	{#if stringKeyvisual.length}
		<p class="warn">
			keyvisual is a string in {stringKeyvisual.join(', ')} – the type expects <code>{'{ image, video? }'}</code>.
			<button type="button" class="secondary" onclick={convertKeyvisual}>Convert</button>
		</p>
	{/if}
	<div class="media">
		<MediaSlot toon={doc.toon} {place} label="thumbnail" value={ch?.thumbnail} onchange={(v) => doc.setShared(['thumbnail'], v)} testid="slot-thumbnail" />
		{#if !stringKeyvisual.length}
			<MediaSlot toon={doc.toon} {place} label="keyvisual" value={ch?.keyvisual?.image} onchange={(v) => doc.setShared(['keyvisual', 'image'], v)} testid="slot-keyvisual" />
			<MediaSlot toon={doc.toon} {place} kind="video" label="keyvisual video" value={ch?.keyvisual?.video} onchange={(v) => doc.setShared(['keyvisual', 'video'], v)} testid="slot-keyvisual-video" />
		{/if}
	</div>
	{#each doc.present.filter((l) => l !== doc.primary) as l}
		{#if doc.get(l, ['thumbnail']) !== ch?.thumbnail}
			<p class="hint">{l} thumbnail: {doc.get(l, ['thumbnail'])} ({m['creator.override']()})
				<button type="button" class="ghost" onclick={() => doc.resetOverride(l, ['thumbnail'])}>{m['creator.use_shared']()}</button></p>
		{/if}
	{/each}

	{@render featuring?.()}
</section>

<style>
	.chapter-head h2 {
		margin: 0;
		font-size: var(--type-title-size, 1.5rem);
		text-transform: none;
		letter-spacing: normal;
	}
	.chapter-head .hint {
		margin: 0.25rem 0 0;
	}
	.meta-panel {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-width: 56rem;
	}
	.media {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem 1rem;
	}
	.warn {
		color: var(--warn);
	}
	.error {
		color: var(--error);
	}
	.status {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		align-items: flex-start;
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
