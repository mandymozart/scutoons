<script lang="ts">
	import { crossAppClick } from '@scutoons/pwa';
	import { onMount } from 'svelte';
	import {
		cdnUrl,
		entities,
		emptyCredits,
		resolveCredits,
		validateCredits,
		validatePeople,
		validateRoleVocabulary,
		type ToonLanguages
	} from '@scutoons/content';
	import { CreditsView } from '@scutoons/reader';
	import { Button, PillTabs } from '@scutoons/ui/editor';
	import { CDN_SERVER, VIEWER_URL } from '$lib/config.ts';
	import { creditsDocs, creditsOf, peopleOf, rolesOf } from '$lib/editor/credits.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import WritingPage from '$lib/components/writing/writing-page.svelte';
	import { useTabs } from '$lib/shell/shell.svelte.ts';
	import { fileLabels } from '$lib/labels.ts';
	import SectionsEditor from '$lib/components/credits/sections-editor.svelte';
	import PeoplePanel from '$lib/components/credits/people-panel.svelte';
	import RolesPanel from '$lib/components/credits/roles-panel.svelte';

	/**
	 * Credits page of a toon: sections of its credits (entity `credits:<toon>`), the global people registry and role
	 * vocabulary (entities `people`, `roles`) and a live preview per toon language (the viewer's CreditsView, a panel). The
	 * lists stand in the writing page's column (lap 2 C14).
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const { credits, people, roles } = creditsDocs(toon);
	const docs = [credits, people, roles];
	const langs = $derived(languages.languages as string[]);
	let tab = $state('sections');
	// svelte-ignore state_referenced_locally
	// the preview language = the header's text language switch (owner 2026-10-03); source / all = the source
	const previewLocale = $derived(textLanguage.chosen(langs) ?? languages.source);
	let selectedPerson = $state<string | null>(null);
	const loaded = $derived(docs.every((d) => !d.loading));
	const missing = $derived(loaded ? docs.filter((d) => !d.present.length) : []);

	onMount(() => {
		for (const d of docs) d.load();
	});

	function createMissing() {
		if (!credits.present.length) credits.create(() => emptyCredits(toon));
		if (!people.present.length) people.create(() => ({ people: [] }));
		if (!roles.present.length) roles.create(() => ({ roles: [] }));
	}

	function validate() {
		const registry = peopleOf(people);
		const vocabulary = rolesOf(roles);
		return [
			...(creditsOf(credits) ? validateCredits(creditsOf(credits), { people: registry, vocabulary, languages: langs, file: entities.credits(toon) }) : []),
			...(registry ? validatePeople(registry, langs, entities.people) : []),
			...(vocabulary ? validateRoleVocabulary(vocabulary, langs, entities.roles) : [])
		];
	}

	const preview = $derived.by(() => {
		const file = creditsOf(credits);
		if (!file || !peopleOf(people)) return null;
		return resolveCredits($state.snapshot(file), $state.snapshot(peopleOf(people)), $state.snapshot(rolesOf(roles)), previewLocale, {
			fallbacks: langs.filter((l) => l !== previewLocale)
		});
	});

	function openPerson(id: string) {
		selectedPerson = id;
		tab = 'people';
	}
	const tabs = $derived([
		{ id: 'sections', label: m['creator.credits_tab_sections'](), testid: 'credits-tab-sections' },
		{ id: 'people', label: `${m['creator.credits_tab_people']()} (${peopleOf(people)?.people.filter((p) => !p.merged_into).length ?? 0})`, testid: 'credits-tab-people' },
		{ id: 'roles', label: m['creator.credits_tab_roles'](), testid: 'credits-tab-roles' }
	]);
	useTabs(() => ({ preview: missing.length || !loaded ? null : previewTab }));
</script>

{#snippet previewTab()}
	<div class="preview" data-testid="credits-preview">
		<p class="live"><a href="{VIEWER_URL}/{toon}/credits" target="_blank" rel="noreferrer" onclick={crossAppClick}>{m['creator.credits_live']()}</a></p>
		<div class="frame" lang={previewLocale}>
			{#if preview}<CreditsView credits={preview} imageUrl={(path) => cdnUrl(CDN_SERVER, path)} />{/if}
		</div>
	</div>
{/snippet}

<MetaPage {toon} title={m['creator.credits']()} {docs} {validate} wide help="credits">
	<WritingPage title={m['creator.credits']()} sub={toon} lists testid="credits-page">
		{#if missing.length}
			<section class="missing" data-testid="credits-missing">
				<p>{m['creator.credits_missing']({ files: fileLabels(missing.map((d) => d.entity)) })}</p>
				<Button icon="plus" onclick={createMissing} data-testid="create-credits">{m['creator.credits_create']()}</Button>
			</section>
		{:else}
			<PillTabs items={tabs} on={tab} onselect={(id) => (tab = id)} label={m['creator.credits']()} />
			<div class="panel">
				{#if tab === 'sections'}
					<SectionsEditor {credits} {people} {roles} languages={langs} onperson={openPerson} />
				{:else if tab === 'people'}
					<PeoplePanel {people} {credits} languages={langs} bind:selected={selectedPerson} />
				{:else}
					<RolesPanel {credits} {roles} languages={langs} />
				{/if}
			</div>
		{/if}
	</WritingPage>
</MetaPage>

<style>
	.panel {
		padding-top: 1.25rem;
	}
	.preview {
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	.live {
		margin: 0;
		padding: 0 var(--editor-pad) var(--editor-pad);
		font-size: var(--editor-small);
	}
	.frame {
		flex: 1 1 auto;
		min-height: 0;
		overflow: auto;
	}
	/* the reader's page at a smaller scale (it is sized in rem of the reader's fluid root) */
	.frame > :global(*) {
		zoom: 0.6;
	}
	.missing {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		align-items: flex-start;
	}
	.missing p {
		margin: 0;
		color: var(--tool-muted);
	}
</style>
