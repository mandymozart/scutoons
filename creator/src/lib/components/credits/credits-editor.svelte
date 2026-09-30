<script lang="ts">
	import { onMount } from 'svelte';
	import {
		cdnUrl,
		CREDITS_FILE,
		CREDITS_SCOPE,
		emptyCredits,
		resolveCredits,
		validateCredits,
		validatePeople,
		validateRoleVocabulary,
		type ToonLanguages
	} from '@scutoons/content';
	import { CreditsView } from '@scutoons/reader';
	import { Tabs } from '@scutoons/ui';
	import { CDN_SERVER, VIEWER_URL } from '$lib/config.ts';
	import { creditsDocs, creditsOf, peopleOf, rolesOf } from '$lib/editor/credits.ts';
	import { m } from '$lib/paraglide/messages.js';
	import MetaPage from '../meta-page.svelte';
	import SectionsEditor from './sections-editor.svelte';
	import PeoplePanel from './people-panel.svelte';
	import RolesPanel from './roles-panel.svelte';

	/**
	 * Credits page of a toon: sections of `<toon>/_/credits.json`, the global people registry and role vocabulary
	 * (`scutoons/_/people.json`, `roles.json`) and a live preview per toon language (the viewer's CreditsView).
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const { credits, people, roles } = creditsDocs(toon);
	const docs = [credits, people, roles];
	const langs = $derived(languages.languages as string[]);
	let tab = $state('sections');
	// svelte-ignore state_referenced_locally
	let previewLocale = $state<string>(languages.source);
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
			...(creditsOf(credits) ? validateCredits(creditsOf(credits), { people: registry, vocabulary, languages: langs, file: CREDITS_FILE }) : []),
			...(registry ? validatePeople(registry, langs, `${CREDITS_SCOPE}/people.json`) : []),
			...(vocabulary ? validateRoleVocabulary(vocabulary, langs, `${CREDITS_SCOPE}/roles.json`) : [])
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
</script>

<MetaPage {toon} title={m['creator.credits']()} {docs} {validate} wide help="credits">
	{#if missing.length}
		<section class="missing" data-testid="credits-missing">
			<p>{m['creator.credits_missing']({ files: missing.map((d) => `${d.toon}/_/${d.path}`).join(', ') })}</p>
			<button type="button" onclick={createMissing} data-testid="create-credits">+ {m['creator.credits_create']()}</button>
		</section>
	{:else}
		<div class="layout">
			<div class="editor">
				<Tabs
					tabs={[
						{ id: 'sections', label: m['creator.credits_tab_sections']() },
						{ id: 'people', label: `${m['creator.credits_tab_people']()} (${peopleOf(people)?.people.filter((p) => !p.merged_into).length ?? 0})` },
						{ id: 'roles', label: m['creator.credits_tab_roles']() }
					]}
					bind:value={tab}
					label={m['creator.credits']()}
				>
					{#snippet children(active)}
						<div class="panel">
							{#if active === 'sections'}
								<SectionsEditor {credits} {people} {roles} languages={langs} onperson={openPerson} />
							{:else if active === 'people'}
								<PeoplePanel {people} {credits} languages={langs} bind:selected={selectedPerson} />
							{:else}
								<RolesPanel {credits} {roles} languages={langs} />
							{/if}
						</div>
					{/snippet}
				</Tabs>
			</div>
			<aside class="preview" data-testid="credits-preview">
				<header>
					<strong>{m['creator.preview']()}</strong>
					{#each langs as l (l)}
						<button type="button" class="ghost mini" class:active={previewLocale === l} onclick={() => (previewLocale = l)} data-testid="preview-{l}">{l}</button>
					{/each}
					<a class="hint" href="{VIEWER_URL}/{toon}/credits" target="_blank" rel="noreferrer">{m['creator.credits_live']()} ↗</a>
				</header>
				<div class="frame" lang={previewLocale}>
					{#if preview}<CreditsView credits={preview} imageUrl={(path) => cdnUrl(CDN_SERVER, path)} />{/if}
				</div>
			</aside>
		</div>
	{/if}
</MetaPage>

<style>
	.layout {
		display: grid;
		grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
		gap: 1rem;
		align-items: start;
	}
	@media (max-width: 1100px) {
		.layout {
			grid-template-columns: 1fr;
		}
	}
	.panel {
		padding-top: 0.75rem;
	}
	.preview {
		position: sticky;
		top: 0.5rem;
		border: 2px solid var(--ink);
		max-height: calc(100vh - 160px);
		display: flex;
		flex-direction: column;
	}
	.preview header {
		display: flex;
		gap: 0.4rem;
		align-items: center;
		padding: 0.3rem 0.5rem;
		border-bottom: 2px solid var(--ink);
	}
	.preview header a {
		margin-left: auto;
	}
	.frame {
		overflow: auto;
	}
	/* the reader's page at a smaller scale (it is sized in rem of the reader's fluid root) */
	.frame > :global(*) {
		zoom: 0.6;
	}
	.mini {
		font-size: 11px;
		padding: 0.1rem 0.4rem;
	}
	.missing {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		align-items: flex-start;
	}
</style>
