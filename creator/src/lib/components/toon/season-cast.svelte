<script lang="ts">
	import { onMount } from 'svelte';
	import { REGULAR_PERSONA, characterIndex, entities, findPersona, personaIds, pickText, toonStar, validateCast, type Problem, type RegistryCharacter, type ToonCast, type ToonLanguages } from '@scutoons/content';
	import { SortGrip, sortable } from '@scutoons/ui';
	import { Button, Chip, Field, IconButton } from '@scutoons/ui/editor';
	import { addToSeasonCast, castDoc, castOf, castWithStar, edit, ensureCast, registryDoc, registryOf } from '$lib/editor/characters.ts';
	import { thingId } from '$lib/editor/library-list.ts';
	import { library } from '$lib/editor/library.svelte.ts';
	import { moveInCast, setCastPersona, setCastRole, takeOutOfCast } from '$lib/editor/season-cast.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import MetaPage from '$lib/components/page/meta-page.svelte';

	/**
	 * A season's cast, edited in the toon's season (lap2 C1): who is in it, in what order (drag), as which personas, their
	 * role per language; Add opens the Library as a pick, ✕ takes one out (never the toon's star). A name opens the
	 * character in the Characters workspace. Saved with `cast:<toon>`.
	 */
	let { toon, number, languages }: { toon: string; number: number; languages: ToonLanguages } = $props();

	// the route re-creates this component per toon / season ({#key})
	// svelte-ignore state_referenced_locally
	const cast = castDoc(toon);
	const registry = registryDoc();
	onMount(() => void Promise.all([cast.load(), registry.load()]));

	const reg = $derived(registryOf(registry));
	const index = $derived(characterIndex(reg));
	const star = $derived(toonStar(reg, toon));
	const entries = $derived(castOf(cast)?.seasons?.[String(number)] ?? []);
	const langs = $derived(languages.languages as string[]);
	const title = $derived(`${fileLabel(entities.season(toon, number))} · ${m['creator.cast']()}`);

	const nameOf = (id: string) => shownCharacterName(index.get(id) ?? null) || id;
	const personaLabel = (c: RegistryCharacter | undefined, p: string) => (p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(c, p)?.label, textLanguage.names, langs) || p);

	function change(fn: (file: ToonCast) => void, key?: string) {
		ensureCast(cast);
		edit<ToonCast>(cast, fn, key);
	}

	/** Add: the Library as a pick of characters – the toon's first, the ones cast here left out */
	function add() {
		library.startPick({
			label: title,
			types: ['character'],
			first: 'toon',
			taken: entries.map((e) => `character:${e.character}`),
			onthing: (picked) => addToSeasonCast(cast, number, thingId(picked.address))
		});
	}

	const validate = (): Problem[] => {
		const file = castOf(cast);
		return file ? validateCast(file, { registry: reg, languages: langs, file: entities.cast(toon) }) : [];
	};
</script>

<MetaPage {toon} help="season" {title} docs={[cast]} {validate} beforeSave={() => void castWithStar(cast, registry, toon, [number])}>
	<section class="cast" data-testid="season-cast">
		<div class="head">
			<span class="label">{title}</span>
			<Button icon="plus" onclick={add} title={m['creator.cast_add_hint']()} data-testid="cast-add">{m['creator.cast_add']()}</Button>
		</div>
		<ul use:sortable={{ onmove: (from, to) => change((f) => void moveInCast(f, number, from, to)) }}>
			{#each entries as e (e.character)}
				{@const c = index.get(e.character) ?? undefined}
				{@const personas = personaIds(c)}
				{@const isStar = e.character === star}
				<li data-sort-row data-testid="cast-{e.character}">
					<SortGrip label={m['creator.sort_move']({ name: nameOf(e.character) })} title={m['creator.cast_order']()} />
					<div class="body">
						<div class="who">
							<a href="/characters?id={encodeURIComponent(e.character)}" title={m['creator.cast_open']()} data-testid="cast-open-{e.character}">{nameOf(e.character)}</a>
							<span class="address">character:{e.character}</span>
						</div>
						<div class="as">
							{#each langs as l (l)}
								<Field
									label={l.toUpperCase()}
									value={e.role?.[l] ?? ''}
									placeholder={m['creator.cast_role_placeholder']()}
									title={m['creator.cast_role']()}
									oninput={(ev) => change((f) => setCastRole(f, number, e.character, l, ev.currentTarget.value), `role-${number}-${e.character}-${l}`)}
									data-testid="cast-role-{e.character}-{l}"
								/>
							{/each}
							<span class="personas" role="group" aria-label={m['creator.cast_personas']()}>
								{#each personas.length ? personas : [REGULAR_PERSONA] as p (p)}
									{@const on = e.personas.includes(p)}
									{@const last = on && e.personas.length === 1}
									<!-- a character with one persona: it is shown, nothing to choose -->
									<Chip
										label={personaLabel(c, p)}
										{on}
										disabled={last && personas.length > 1}
										title={last && personas.length > 1 ? m['creator.cast_last_persona']() : m['creator.cast_personas']()}
										onclick={personas.length > 1 ? () => change((f) => setCastPersona(f, number, e.character, p, !on)) : undefined}
									/>
								{/each}
							</span>
						</div>
					</div>
					<IconButton
						icon="x"
						tone="muted"
						label={isStar ? m['creator.cast_star_locked']() : m['creator.cast_leave']()}
						disabled={isStar}
						onclick={() => change((f) => takeOutOfCast(f, number, e.character, star))}
						data-testid="cast-leave-{e.character}"
					/>
				</li>
			{/each}
		</ul>
	</section>
</MetaPage>

<style>
	.cast {
		max-width: 46rem;
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 0.4rem;
	}
	.label {
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	ul {
		margin: 0;
		padding: 0;
		list-style: none;
		border-top: var(--editor-edge) solid var(--tool-line);
	}
	li {
		display: flex;
		align-items: flex-start;
		gap: 0.6rem;
		padding: 0.55rem 0;
		border-bottom: var(--editor-edge) solid var(--tool-line);
	}
	li > :global(.sc-sort-grip) {
		margin-top: 0.2rem;
	}
	.body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.who {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
	}
	.who a {
		color: var(--tool-text);
		text-decoration: underline;
		text-underline-offset: 0.2em;
	}
	.address {
		color: var(--tool-muted);
		font-family: var(--editor-sans);
		font-size: var(--editor-small);
	}
	.as {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem 0.75rem;
	}
	.as > :global(.kit-field) {
		grid-template-columns: auto 9rem;
		gap: 0.5rem;
	}
	.personas {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
</style>
