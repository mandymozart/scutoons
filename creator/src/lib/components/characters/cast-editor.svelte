<script lang="ts">
	import { onMount } from 'svelte';
	import {
		AUTHORED_VIEW,
		entities,
		REGULAR_PERSONA,
		castSeasons,
		characterImage,
		characterIndex,
		characterName,
		personaIds,
		pickText,
		findPersona,
		toonStar,
		validateCast,
		validateCharacters,
		type LocaleText,
		type Problem,
		type ToonCast,
		type ToonLanguages
	} from '@scutoons/content';
	import {
		addCharacter,
		addToSeasonCast,
		castDoc,
		castWithStar,
		castOf,
		edit,
		ensureCast,
		loadToons,
		moodsDoc,
		moodsOf,
		registryDoc,
		registryOf,
		type ToonInfo,
		isKnownMissing,
		imageExists
	} from '$lib/editor/characters.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import MetaPage from '../meta-page.svelte';
	import LocaleMapField from '../credits/locale-map-field.svelte';
	import CharacterPickerSheet from './character-picker-sheet.svelte';

	/**
	 * The cast of a toon per season (entity `cast:<toon>`): registry characters with the personas that appear in that
	 * season, in order, with a role per toon language. Add from the whole registry (visual picker: character → persona) or
	 * create a character inline (a new registry entry); reorder by drag or ArrowUp/ArrowDown. Saved / published together
	 * with the registry (a new character is a registry change).
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const cast = castDoc(toon);
	const registry = registryDoc();
	const moods = moodsDoc();
	const docs = [cast, registry];
	const langs = $derived(languages.languages as string[]);
	const source = $derived(languages.source as string);
	let toons = $state<ToonInfo[]>([]);
	let season = $state(1);
	let pickerOpen = $state(false);
	let newName = $state('');
	let dragging = $state<number | null>(null);

	onMount(async () => {
		await Promise.all([cast.load(), registry.load(), moods.load()]);
		const seasons = castSeasons(castOf(cast));
		if (seasons.length) season = seasons[0];
		// the star is in every season (an older cast without it is completed here, saved with the next save)
		castWithStar(cast, registry, toon, [season]);
		toons = await loadToons(getLocale());
	});

	/** the toon's star: always in the cast, cannot be removed */
	const star = $derived(toonStar(registryOf(registry), toon));

	const file = $derived(castOf(cast));
	const reg = $derived(registryOf(registry));
	const index = $derived(characterIndex(reg));
	// the seasons are created on the toon page; the cast lists the ones it has (and season 1)
	const seasons = $derived([...new Set([1, ...castSeasons(file), season])].sort((a, b) => a - b));
	const entries = $derived(file?.seasons?.[String(season)] ?? []);

	function change(fn: (f: ToonCast) => void, key?: string) {
		ensureCast(cast);
		edit<ToonCast>(cast, fn, key);
	}
	const list = (f: ToonCast) => (f.seasons[String(season)] ??= []);
	function move(from: number, to: number) {
		if (to < 0 || to >= entries.length || from === to) return;
		change((f) => {
			const l = list(f);
			const [item] = l.splice(from, 1);
			l.splice(to, 0, item);
		});
	}
	function togglePersona(character: string, persona: string, on: boolean) {
		change((f) => {
			const entry = list(f).find((e) => e.character === character);
			if (!entry) return;
			entry.personas = on ? [...new Set([...entry.personas, persona])] : entry.personas.filter((p) => p !== persona);
			if (!entry.personas.length) entry.personas = [REGULAR_PERSONA];
		});
	}
	function setRole(character: string, value: LocaleText) {
		change((f) => {
			const entry = list(f).find((e) => e.character === character);
			if (!entry) return;
			if (Object.values(value).some((t) => t?.trim())) entry.role = value;
			else delete entry.role;
		}, `role-${season}-${character}`);
	}
	function remove(character: string) {
		if (character === star) return;
		change((f) => (f.seasons[String(season)] = list(f).filter((e) => e.character !== character)));
	}
	function createInline() {
		if (!newName.trim()) return;
		const id = addCharacter(registry, newName, langs);
		addToSeasonCast(cast, season, id);
		newName = '';
	}

	function validate(): Problem[] {
		const out: Problem[] = [];
		if (file) out.push(...validateCast(file, { registry: reg, languages: langs, file: entities.cast(toon) }));
		if (reg) {
			// the registry's problems of this toon's characters only
			const ids = new Set(castSeasons(file).flatMap((s) => (file?.seasons[String(s)] ?? []).map((e) => e.character)));
			out.push(
				...validateCharacters(reg, { languages: langs, moods: moodsOf(moods), file: entities.characters }).filter((p) => {
					const i = p.path[1];
					return typeof i === 'number' && ids.has(reg.characters[i]?.id);
				})
			);
		}
		return out;
	}
</script>

<MetaPage {toon} title={m['creator.cast']()} {docs} {validate} help="cast" beforeSave={() => castWithStar(cast, registry, toon, [season])}>
	{#snippet actions()}
		<a class="bar-link" href="/characters?toon={toon}" data-testid="open-characters">{m['creator.characters']()}</a>
	{/snippet}
	<section class="cast" data-testid="cast-editor">
		<nav class="seasons" aria-label={m['creator.cast_seasons']()}>
			{#each seasons as s (s)}
				<button type="button" class="tab" class:active={s === season} onclick={() => (season = s)} data-testid="cast-season-{s}">{m['creator.season']({ number: s })}</button>
			{/each}
		</nav>
		{#if cast.loading || registry.loading}
			<p class="hint">{m['creator.loading']()}</p>
		{:else}
			<ol>
				{#each entries as entry, i (entry.character)}
					{@const c = index.get(entry.character)}
					{@const src = c ? characterImage(c, { persona: entry.personas[0] }, AUTHORED_VIEW, { isMissing: isKnownMissing }) : ''}
					<li
						class="member"
						class:dragging={dragging === i}
						draggable="true"
						ondragstart={() => (dragging = i)}
						ondragover={(e) => e.preventDefault()}
						ondrop={() => {
							if (dragging !== null) move(dragging, i);
							dragging = null;
						}}
						ondragend={() => (dragging = null)}
						data-testid="cast-member-{entry.character}"
					>
						<button
							type="button"
							class="ghost handle"
							aria-label={m['creator.credits_move']({ name: entry.character })}
							title={m['creator.credits_move_hint']()}
							onkeydown={(e) => {
								if (e.key === 'ArrowUp') {
									e.preventDefault();
									move(i, i - 1);
								} else if (e.key === 'ArrowDown') {
									e.preventDefault();
									move(i, i + 1);
								}
							}}>⠿</button
						>
						<span class="avatar" style:background={c?.color || 'transparent'}>{#if src}<img src={asset(src)} onerror={() => imageExists(src)} alt="" />{/if}</span>
						<span class="who">
							{#if c}
								<strong>{characterName(c, source)}{#if c.id === star} <span class="star" title={m['creator.cast_star_locked']()} data-testid="cast-star">{m['creator.cast_star']()}</span>{/if}</strong>
								<a class="hint" href="/characters?id={encodeURIComponent(c.id)}&toon={toon}">{m['creator.characters_edit']()}</a>
								<span class="personas" data-testid="cast-personas-{entry.character}">
									{#each personaIds(c) as p (p)}
										<label class="chip" class:on={entry.personas.includes(p)}>
											<input type="checkbox" checked={entry.personas.includes(p)} onchange={(e) => togglePersona(entry.character, p, e.currentTarget.checked)} />
											{p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(c, p)?.label, source, langs) || p}
										</label>
									{/each}
								</span>
							{:else}
								<strong class="bad">{entry.character}</strong>
								<span class="hint bad">{m['creator.characters_not_in_registry']()}</span>
							{/if}
						</span>
						<LocaleMapField value={entry.role} languages={langs} label={m['creator.cast_role']()} onchange={(v) => setRole(entry.character, v)} testid="cast-role-{entry.character}" />
						<span class="order">
							<button type="button" class="ghost" onclick={() => move(i, i - 1)} disabled={i === 0} aria-label={m['creator.credits_up']()}>↑</button>
							<button type="button" class="ghost" onclick={() => move(i, i + 1)} disabled={i === entries.length - 1} aria-label={m['creator.credits_down']()}>↓</button>
							{#if entry.character === star}
								<button type="button" class="ghost" disabled title={m['creator.cast_star_locked']()} aria-label={m['creator.cast_star_locked']()} data-testid="cast-star-locked">✕</button>
							{:else}
								<button type="button" class="ghost" onclick={() => remove(entry.character)} aria-label={m['creator.credits_remove']()}>✕</button>
							{/if}
						</span>
					</li>
				{:else}
					<li class="hint">{m['creator.cast_empty']()}</li>
				{/each}
			</ol>
			<div class="add">
				<button type="button" class="secondary" onclick={() => (pickerOpen = true)} data-testid="cast-add">+ {m['creator.cast_add']()}</button>
				<span class="new">
					<input type="text" bind:value={newName} placeholder={m['creator.characters_new_character']()} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), createInline())} data-testid="cast-new-name" />
					<button type="button" class="ghost" onclick={createInline} disabled={!newName.trim()} data-testid="cast-new-create">{m['creator.characters_create']()}</button>
				</span>
			</div>
		{/if}
	</section>
</MetaPage>

<CharacterPickerSheet
	bind:open={pickerOpen}
	registry={reg}
	moods={moodsOf(moods)}
	{toons}
	cast={entries}
	season={m['creator.season']({ number: season })}
	locale={source}
	mode="persona"
	onpick={(p) => addToSeasonCast(cast, season, p.character, p.persona)}
/>

<style>
	.cast {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.seasons {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		border-bottom: 2px solid var(--ink);
		padding-bottom: 0.25rem;
	}
	.tab {
		background: var(--paper);
		color: var(--ink);
		border: 2px solid var(--border-muted);
		padding: 0.25rem 0.6rem;
		font-size: 12px;
	}
	.tab.active {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.member {
		display: grid;
		grid-template-columns: auto 3.2rem minmax(12rem, 18rem) minmax(0, 1fr) auto;
		gap: 0.6rem;
		align-items: center;
		border: 1px solid var(--border-subtle);
		padding: 0.4rem;
		background: var(--paper);
	}
	.member.dragging {
		opacity: 0.5;
	}
	.handle {
		cursor: grab;
	}
	.avatar {
		width: 3.2rem;
		height: 3.2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.avatar img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.who {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 12px;
	}
	.personas {
		display: flex;
		flex-wrap: wrap;
		gap: 0.2rem;
	}
	.chip {
		display: inline-flex;
		gap: 0.2rem;
		align-items: center;
		font-size: 10px;
		border: 1px solid var(--border-muted);
		padding: 0 0.3rem;
		cursor: pointer;
	}
	.chip.on {
		border-color: var(--ink);
		background: var(--primary-cyan);
	}
	.order {
		display: flex;
		align-items: center;
	}
	.star {
		margin-left: 0.4rem;
		font-size: 10px;
		font-weight: 400;
		padding: 0 0.4em;
		border: 1px solid var(--ink);
		background: var(--primary-cyan);
		vertical-align: middle;
	}

	.bad {
		color: var(--error);
	}
	.bar-link {
		color: var(--ink);
		white-space: nowrap;
		font-size: 12px;
	}
	.add {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		align-items: center;
	}
	.new {
		display: flex;
		gap: 0.3rem;
	}
	@media (max-width: 800px) {
		.member {
			grid-template-columns: auto 3.2rem 1fr auto;
		}
	}
</style>
