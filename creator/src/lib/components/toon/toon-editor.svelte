<script lang="ts">
	import { Select, Toggle } from '@scutoons/ui/editor';
	import { entities, resolveText, type Status, type ToonLanguages } from '@scutoons/content';
	import { MediaSlot } from '@scutoons/media';
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
	 * thumbnail, the avatar + the landing tile, announce, the map's start view. An entry without texts yet gets them from the old
	 * per-language toon settings on load (a notice; saving stores them in the entry). Covers are per season (owner 2026-10-02; the season's cover editor) – the toon is shown with its avatar.
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const entry = toonIndexDoc(toon, languages);
	const nav = getCreatorNav();

	const data = $derived(entry.get(entry.primary, []));
	/** the status per language follows the header's text language like the texts (source, one language, or all) */
	const statusLocales = $derived(textLanguage.visible(entry.textLanguages, entry.textSource));
	/** a language's status: its own, else the source's (the reader's rule) – null: none at all (= published) */
	const statusOf = (locale: string) => resolveText<Status>(data?.languageStatus, [locale, entry.textSource, ...entry.textLanguages]) ?? null;
	const ownStatus = (locale: string) => entry.text(['languageStatus'], locale) !== undefined;

	onMount(async () => {
		await entry.load();
	});
</script>

<MetaPage {toon} title={fileLabel(entities.toonSettings(toon))} docs={[entry]} help="toon-file">
	{#if entry.loading}
		<p>{m['creator.loading']()}</p>
	{:else if entry.present.length === 0}
		<p class="warn">{m['creator.toon_entry_missing']()} <a href="/{toon}">{nav.toonName(toon)}</a></p>
	{:else}
		<section class="sc-section" data-testid="toon-meta">
			<h2>{m['creator.meta']()}</h2>
			<LocaleText doc={entry} path={['title']} label={m['creator.field_title']()} multiline={false} />
			<LocaleText doc={entry} path={['description']} label={m['creator.field_description']()} rows={3} />
			<div class="status" title={m['creator.toon_status_hint']()} data-testid="toon-status">
				<span class="sc-label">{m['creator.status']()}</span>
				<div class="status-row">
					{#each statusLocales as l (l)}
						<span class="status-field" class:inherited={!ownStatus(l)}>
							<span>{l}{#if l === entry.textSource}<SourceMark />{/if}</span>
							<Select
								quiet
								label="{m['creator.status']()} · {l}"
								value={statusOf(l) ?? 'published'}
								options={STATUSES.map((s) => ({ value: s, label: statusWord(s) }))}
								onchange={(v) => entry.setText(['languageStatus'], l, v, `status:${l}`)}
								data-testid="toon-status-{l}"
							/>
						</span>
					{/each}
				</div>
			</div>
			<LocaleText doc={entry} path={['release']} label={m['creator.field_release']()} type="date" />
		</section>

		<section class="sc-section neutral" data-testid="toon-neutral">
			<h2 title={m['creator.toon_neutral_hint']()}>{m['creator.toon_neutral']()}</h2>
			<PropField doc={entry} path={['authors']} label={m['creator.field_authors']()} type="list" testid="toon-authors" />
			<div class="sc-fields" data-cols="3" data-align="start">
				<PropField doc={entry} path={['chapters']} label={m['creator.field_chapter_count']()} type="number" step={1} min={0} />
			</div>
			<div class="media">
				<MediaSlot {toon} label={m['creator.field_thumbnail']()} value={data?.thumbnail || undefined} onchange={(v) => entry.set(['thumbnail'], v || undefined)} testid="toon-thumbnail" />
			</div>

			<div class="sub" data-testid="toon-landing">
				<h3 title={m['creator.landing_hint']()}>{m['creator.landing']()}</h3>
				<Toggle shown on={data?.announce === true} label={m['creator.announce']()} title={m['creator.announce_hint']()} onchange={(on) => entry.set(['announce'], on ? true : undefined, 'announce')} data-testid="toon-announce" />
				<div class="media">
					<MediaSlot {toon} label={m['creator.field_avatar']()} value={data?.avatar || undefined} onchange={(v) => entry.set(['avatar'], v || 'avatar.webp')} testid="toon-avatar" />
					<MediaSlot {toon} label={m['creator.field_tile_image']()} value={data?.tile?.image || undefined} onchange={(v) => entry.set(['tile', 'image'], v || undefined)} testid="toon-tile-image" />
				</div>
				<div class="sc-fields" data-cols="3" data-align="start">
					<PropField doc={entry} path={['tile', 'color']} label={m['creator.field_tile_color']()} placeholder={m['creator.tile_color_placeholder']()} testid="toon-tile-color" />
				</div>
			</div>

			<div class="sub" data-testid="toon-map">
				<h3 title={m['creator.map_start_view_hint']()}>{m['creator.map_start_view']()}</h3>
				<a href="/map?toon={toon}">{m['creator.map_open_builder']()}</a>
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
	/* a language without its own status shows the source's */
	.inherited :global(.kit-select) {
		color: var(--tool-muted);
	}
	.warn {
		color: var(--tool-warn);
	}
</style>
