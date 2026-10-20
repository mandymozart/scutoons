<script lang="ts" module>
	export type HelpPage = 'home' | 'toon' | 'chapter' | 'season' | 'toon-file' | 'cover' | 'credits' | 'characters' | 'cast' | 'script' | 'guide' | 'map' | 'catalog' | 'storyboard';
	export type ChapterSection = 'meta' | 'audio' | 'group' | 'slice';
</script>

<script lang="ts">
	import { fileLabel } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import StyleGuide from '$lib/components/page/style-guide.svelte';
	import ScriptRules from '$lib/components/script/script-rules.svelte';

	/**
	 * The help of every creator page, shown in the ActionBar's Info panel (@scutoons/ui `InfoPanel`, owner 2026-09-30):
	 * explanations, conventions and glossaries live here instead of long hints above the fields; only short labels
	 * and errors stay inline. `section`: the chapter editor's current tree section. `seasonFile`: the season's entity.
	 */
	let { page, section = 'meta', seasonFile = '' }: { page: HelpPage; section?: ChapterSection; seasonFile?: string } = $props();
</script>

{#snippet saving()}
	<h3>{m['creator.help_saving_title']()}</h3>
	<p>{m['creator.help_saving']()}</p>
	<p>{m['creator.help_problems']()}</p>
{/snippet}

{#snippet visibility()}
	<h3>{m['creator.help_visibility_title']()}</h3>
	<p>{m['creator.status_hint']({ file: seasonFile ? fileLabel(seasonFile) : m['creator.season']({ number: 1 }) })}</p>
	<p>{m['creator.status_where']()}</p>
{/snippet}

<div class="page-help" data-testid="page-help-{page}">
	{#if page === 'home'}
		<p>{m['creator.help_home']()}</p>
		<h3>{m['creator.characters']()}</h3>
		<p>{m['creator.characters_home_hint']()}</p>
		{@render saving()}
	{:else if page === 'catalog'}
		<p>{m['creator.catalog_help']()}</p>
		<p>{m['creator.catalog_help_physical']()}</p>
		{@render saving()}
	{:else if page === 'storyboard'}
		<p>{m['storyboard.help']()}</p>
		<p>{m['storyboard.help_workspace']()}</p>
		{@render saving()}
	{:else if page === 'map'}
		<p>{m['creator.map_help']()}</p>
		<p>{m['creator.map_help_ids']()}</p>
		<p>{m['creator.map_help_images']()}</p>
		{@render saving()}
	{:else if page === 'toon'}
		<h3>{m['creator.languages']()}</h3>
		<p>{m['creator.languages_hint']()}</p>
		<p>{m['creator.languages_remove_note']()}</p>
		{@render visibility()}
		<h3>{m['creator.help_links_title']()}</h3>
		<ul>
			<li>{m['creator.credits_open']()}: {m['creator.credits_open_hint']()}</li>
			<li>{m['creator.cast']()}: {m['creator.cast_open_hint']()}</li>
			<li>{m['creator.characters']()}: {m['creator.characters_home_hint']()}</li>
		</ul>
		{@render saving()}
	{:else if page === 'chapter'}
		<p>{m['creator.help_chapter_tree']()}</p>
		{#if section === 'meta'}
			{@render visibility()}
		{:else if section === 'audio'}
			<h3>{m['creator.audio']()}</h3>
			<p>{m['creator.help_audio_chapter']()}</p>
		{:else if section === 'group'}
			<h3>{m['creator.help_group_title']()}</h3>
			<p>{m['creator.help_group']()}</p>
		{:else}
			<h3>{m['creator.help_canvas_title']()}</h3>
			<p>{m['creator.help_canvas']()}</p>
			<p>{m['creator.draw_hint']()}</p>
			<h3>{m['creator.help_media_title']()}</h3>
			<p>{m['creator.format_hint']()}</p>
			<p>{m['creator.video_hint']()}</p>
			<h3>{m['creator.audio']()}</h3>
			<p>{m['creator.help_audio_slice']()}</p>
		{/if}
		{@render saving()}
		<h3 class="guide-head">{m['creator.style_guide']()}</h3>
		<StyleGuide />
	{:else if page === 'season'}
		<h3>{m['creator.in_the_reader']()}</h3>
		<p>{m['creator.reader_uses_season']()}</p>
		{@render visibility()}
		{@render saving()}
	{:else if page === 'toon-file'}
		<h3>{m['creator.in_the_reader']()}</h3>
		<p>{m['creator.reader_uses_toon']()}</p>
		{@render saving()}
	{:else if page === 'cover'}
		<p>{m['creator.cover_help']()}</p>
		<p>{m['creator.cover_mode_hint_custom']()}</p>
		{@render saving()}
	{:else if page === 'credits'}
		<p>{m['creator.help_credits']()}</p>
		<h3>{m['creator.credits_lead']()}</h3>
		<p>{m['creator.credits_lead_hint']()}</p>
		<h3>{m['creator.credits_tab_people']()}</h3>
		<p>{m['creator.credits_registry_hint']()}</p>
		<h3>{m['creator.credits_tab_roles']()}</h3>
		<ul>
			<li>{m['creator.credits_custom_roles']()}: {m['creator.credits_custom_roles_hint']()}</li>
			<li>{m['creator.credits_vocabulary']()}: {m['creator.credits_vocabulary_hint']()}</li>
		</ul>
		<p>{m['creator.sort_hint']()}</p>
		{@render saving()}
	{:else if page === 'characters'}
		<h3>{m['creator.characters_glossary']()}</h3>
		<dl data-testid="characters-glossary">
			<dt>{m['creator.characters_character']()}</dt>
			<dd>{m['creator.characters_glossary_character']()}</dd>
			<dt>{m['creator.characters_persona']()}</dt>
			<dd>{m['creator.characters_glossary_persona']()}</dd>
			<dt>{m['creator.characters_look']()}</dt>
			<dd>{m['creator.characters_glossary_look']()}</dd>
			<dt>{m['creator.characters_mood']()}</dt>
			<dd>{m['creator.characters_glossary_mood']()}</dd>
			<dt>{m['creator.characters_view']()}</dt>
			<dd>{m['creator.characters_glossary_view']()}</dd>
		</dl>
		<p>{m['creator.characters_glossary_fallback']()}</p>
		<h3>{m['creator.characters_moods']()}</h3>
		<p>{m['creator.characters_moods_hint']()}</p>
		<h3>{m['creator.characters_wanted']()}</h3>
		<p>{m['creator.characters_wanted_hint']()}</p>
		<h3>{m['creator.help_transitions_title']()}</h3>
		<p>{m['creator.characters_transition_hint']()}</p>
		<p>{m['creator.characters_apose_hint']()}</p>
		{@render saving()}
		<h3 class="guide-head">{m['creator.style_guide']()}</h3>
		<StyleGuide sections={['characters']} />
	{:else if page === 'cast'}
		<p>{m['creator.cast_hint']()}</p>
		<p>{m['creator.cast_star_hint']()}</p>
		{@render saving()}
	{:else if page === 'script'}
		<p>{m['creator.help_script']()}</p>
		<h3>{m['creator.script_rules']()}</h3>
		<ScriptRules />
		{@render saving()}
	{:else if page === 'guide'}
		<p>{m['creator.help_guide']()}</p>
	{/if}
</div>

<style>
	.guide-head {
		padding-top: 0.75rem;
		border-top: 2px solid var(--ink);
	}
</style>
