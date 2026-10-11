<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { entities, mediaRequests, resolveText, type Status, type ToonLanguages } from '@scutoons/content';
	import { MediaSlot } from '@scutoons/media';
	import { requestsDoc, requestsOf } from '$lib/editor/characters.ts';
	import { onMount } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { toonIndexDoc } from '$lib/editor/languages.ts';
	import { STATUSES } from '$lib/editor/visibility.ts';
	import { statusWord } from '$lib/editor/status-word.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import SourceMark from '$lib/components/text/source-mark.svelte';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import { fileLabel } from '$lib/labels.ts';
	import LocaleText from '$lib/components/fields/locale-text.svelte';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import { getCreatorNav } from '$lib/nav.svelte.ts';

	/**
	 * The toon's settings: ONE document, the toon entry `toon:<toon>` (agents/NEUTRAL.md step 4). Per language: title /
	 * description and release (the header's text language, like a chapter's), the status (the toon page of a language
	 * that isn't published is closed in it); the same in every language (`TOON_NEUTRAL_FIELDS`): authors, chapter count,
	 * thumbnail, the landing tile, announce, the map's start view. An entry without texts yet gets them from the old
	 * per-language toon settings on load (a notice; saving stores them in the entry). Plus the toon's open media requests
	 * (wanted images). Covers are per season (owner 2026-10-02; the season's cover editor) – the toon is shown with its avatar.
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const entry = toonIndexDoc(toon, languages);
	const requests = requestsDoc();
	const nav = getCreatorNav();

	const wanted = $derived(mediaRequests(requestsOf(requests), { toon }));
	const data = $derived(entry.get(entry.primary, []));
	/** the status per language follows the header's text language like the texts (source, one language, or all) */
	const statusLocales = $derived(textLanguage.visible(entry.textLanguages, entry.textSource));
	/** a language's status: its own, else the source's (the reader's rule) – null: none at all (= published) */
	const statusOf = (locale: string) => resolveText<Status>(data?.languageStatus, [locale, entry.textSource, ...entry.textLanguages]) ?? null;
	const ownStatus = (locale: string) => entry.text(['languageStatus'], locale) !== undefined;

	onMount(async () => {
		await Promise.all([entry.load(), requests.load().catch(() => {})]);
	});
</script>

<MetaPage {toon} title={fileLabel(entities.toonSettings(toon))} docs={[entry]} help="toon-file" tree={() => ({ scope: { kind: 'toon', toon }, title: m['creator.publish_tree_title']({ name: toon }) })}>
	{#if entry.loading}
		<p>{m['creator.loading']()}</p>
	{:else if entry.present.length === 0}
		<p class="warn">{m['creator.toon_entry_missing']()} <a href="/{toon}">{nav.toonName(toon)}</a></p>
	{:else}
		<section class="block" data-testid="toon-meta">
			<h2>{m['creator.meta']()}</h2>
			<LocaleText doc={entry} path={['title']} label={m['creator.field_title']()} multiline={false} />
			<LocaleText doc={entry} path={['description']} label={m['creator.field_description']()} rows={3} />
			<div class="status" data-testid="toon-status">
				<span class="sc-label">{m['creator.status']()}</span>
				<div class="status-row">
					{#each statusLocales as l (l)}
						<label class="status-field">
							<span>{l}{#if l === entry.textSource}<SourceMark />{/if}</span>
							<select
								class:inherited={!ownStatus(l)}
								value={statusOf(l) ?? 'published'}
								onchange={(e) => entry.setText(['languageStatus'], l, e.currentTarget.value, `status:${l}`)}
								data-testid="toon-status-{l}"
							>
								{#each STATUSES as s}<option value={s}>{statusWord(s)}</option>{/each}
							</select>
						</label>
					{/each}
				</div>
				<p class="hint">{m['creator.toon_status_hint']()}</p>
			</div>
			<LocaleText doc={entry} path={['release']} label={m['creator.field_release']()} type="date" />
		</section>

		<section class="block neutral" data-testid="toon-neutral">
			<h2>{m['creator.toon_neutral']()}</h2>
			<p class="hint">{m['creator.toon_neutral_hint']()}</p>
			<PropField doc={entry} path={['authors']} label={m['creator.field_authors']()} type="list" testid="toon-authors" />
			<div class="sc-fields" data-cols="3" data-align="start">
				<PropField doc={entry} path={['chapters']} label={m['creator.field_chapter_count']()} type="number" step={1} min={0} />
			</div>
			<div class="media">
				<MediaSlot {toon} label={m['creator.field_thumbnail']()} value={data?.thumbnail || undefined} onchange={(v) => entry.set(['thumbnail'], v || undefined)} testid="toon-thumbnail" />
			</div>

			{#if wanted.length}
				<div class="sub" data-testid="toon-media-requests">
					<h3>{m['creator.cover_requests']()}</h3>
					<ul class="wanted">
						{#each wanted as r (r.id)}
							<li><code>{r.path}</code>{#if r.size} <span class="hint">{r.size}</span>{/if}{#if r.note} <span class="sc-mark"><Icon name="mail" /> {r.note}</span>{/if}</li>
						{/each}
					</ul>
				</div>
			{/if}

			<div class="sub" data-testid="toon-landing">
				<h3>{m['creator.landing']()}</h3>
				<p class="hint">{m['creator.landing_hint']()}</p>
				<label class="check">
					<input
						type="checkbox"
						checked={data?.announce === true}
						onchange={(e) => entry.set(['announce'], e.currentTarget.checked ? true : undefined, 'announce')}
						data-testid="toon-announce"
					/>
					<span>{m['creator.announce']()}</span>
				</label>
				<p class="hint">{m['creator.announce_hint']()}</p>
				<div class="media">
					<MediaSlot {toon} label={m['creator.field_tile_image']()} value={data?.tile?.image || undefined} onchange={(v) => entry.set(['tile', 'image'], v || undefined)} testid="toon-tile-image" />
				</div>
				<div class="sc-fields" data-cols="3" data-align="start">
					<PropField doc={entry} path={['tile', 'color']} label={m['creator.field_tile_color']()} placeholder={m['creator.tile_color_placeholder']()} testid="toon-tile-color" />
				</div>
			</div>

			<div class="sub" data-testid="toon-map">
				<h3>{m['creator.map_start_view']()}</h3>
				<p class="hint">{m['creator.map_start_view_hint']()} <a href="/map?toon={toon}">{m['creator.map_open_builder']()}</a></p>
				<div class="sc-fields" data-cols="4" data-align="start">
					<PropField doc={entry} path={['map', 'center', 'x']} label={m['creator.field_map_x']()} type="number" step={1} />
					<PropField doc={entry} path={['map', 'center', 'y']} label={m['creator.field_map_y']()} type="number" step={1} />
					<PropField doc={entry} path={['map', 'zoom']} label={m['creator.field_map_zoom']()} type="number" />
				</div>
			</div>
		</section>
	{/if}
</MetaPage>

<style>
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 2rem;
	}
	.block:last-child {
		margin-bottom: 0;
	}
	.sub {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-top: 0.75rem;
	}
	h2 {
		margin: 0;
	}
	h3 {
		margin: 0;
	}
	.wanted {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}
	/* the field grids (tools.css .sc-fields): two columns below 900px */
	@media (max-width: 900px) {
		.sc-fields[data-cols] {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	.media {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem 1rem;
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
	/* a language without its own status shows the source's */
	.status-field select.inherited {
		color: var(--text-muted);
	}
	.warn {
		color: var(--warn);
	}
	.hint {
		margin: 0;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
</style>
