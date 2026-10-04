<script lang="ts" module>
	/** What the picker returns: a character + persona (+ look + mood in `expression` mode). */
	export interface PickedCharacter {
		character: string;
		persona: string;
		look?: string;
		mood?: string;
	}
</script>

<script lang="ts">
	import { gridArrows, Tabs } from '@scutoons/ui';
	import { isKnownMissing, imageExists } from '$lib/editor/characters.ts';
	import PickerSheet from '$lib/components/controls/picker-sheet.svelte';
	import {
		AUTHORED_VIEW,
		DEFAULT_MOOD,
		REGULAR_PERSONA,
		characterImage,
		characterName,
		defaultLookId,
		findPersona,
		lookImage,
		lookLabel,
		lookMoods,
		moodLabel,
		personaIds,
		pickText,
		searchCharacters,
		sortedMoods,
		type CastEntry,
		type CharacterRegistry,
		type MoodVocabulary,
		type RegistryCharacter
	} from '@scutoons/content';
	import type { ToonInfo } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The visual character picker (every character field): step 1 a searchable grid of ALL registry characters (the
	 * season's cast first), step 2 the chosen character's personas → looks → a grid of portrait moods; clicking a cell
	 * picks character + persona + look + mood at once (`expression` mode) – or a persona card (`persona` mode, casts).
	 * Keyboard: arrows move in the grids, Enter picks, Escape closes.
	 */
	let {
		open = $bindable(false),
		registry,
		moods,
		toons = [],
		cast = [],
		season,
		value,
		locale,
		mode = 'expression',
		onpick
	}: {
		open?: boolean;
		registry: CharacterRegistry | null;
		moods: MoodVocabulary;
		toons?: ToonInfo[];
		/** the season's cast (shown first; picking someone else adds them – the caller does that) */
		cast?: CastEntry[];
		/** e.g. "Season 1" (section title) */
		season?: string;
		value?: Partial<PickedCharacter>;
		locale: string;
		mode?: 'expression' | 'persona';
		onpick: (picked: PickedCharacter) => void;
	} = $props();

	let step = $state<1 | 2>(1);
	let chosen = $state<string | null>(null);
	let persona = $state(REGULAR_PERSONA);
	let look = $state('');
	let query = $state('');
	let toonFilter = $state('');
	let missingOnly = $state(false);
	let allMoods = $state(false);
	let grid = $state<HTMLElement>();

	// every time it opens: start at the current value (its character → step 2)
	$effect(() => {
		if (!open) return;
		query = '';
		if (value?.character && registry?.characters.some((c) => c.id === value.character)) {
			chosen = value.character;
			persona = value.persona || REGULAR_PERSONA;
			look = value.look ?? '';
			step = 2;
		} else {
			chosen = null;
			step = 1;
		}
	});

	const characters = $derived((registry?.characters ?? []).filter((c) => !!c?.id));
	const castIds = $derived(new Set(cast.map((e) => e.character)));
	const missingStates = (c: RegistryCharacter) => c.personas.some((p) => p.looks.some((l) => !lookImage(l, DEFAULT_MOOD, AUTHORED_VIEW)));
	const matches = $derived(
		searchCharacters(characters, query, 10000).filter(
			(c) =>
				(!toonFilter || c.own_toon === toonFilter) &&
				(!missingOnly || missingStates(c))
		)
	);
	const sections = $derived(
		[
			{ id: 'cast', title: season ? m['creator.characters_picker_cast']({ season }) : '', list: matches.filter((c) => castIds.has(c.id)) },
			{ id: 'all', title: m['creator.characters_picker_all'](), list: matches.filter((c) => !castIds.has(c.id)) }
		].filter((s) => s.list.length)
	);

	const character = $derived(characters.find((c) => c.id === chosen) ?? null);
	const currentPersona = $derived(findPersona(character, persona) ?? findPersona(character, REGULAR_PERSONA));
	const lookId = $derived(currentPersona?.looks.some((l) => l.id === look) ? look : defaultLookId(currentPersona));
	const currentLook = $derived(currentPersona?.looks.find((l) => l.id === lookId));
	const rowMoods = $derived.by(() => {
		const own = lookMoods(currentLook, moods);
		if (!allMoods) return own;
		return [...own, ...sortedMoods(moods).map((x) => x.id).filter((x) => !own.includes(x))];
	});
	const castPersonas = $derived(cast.find((e) => e.character === chosen)?.personas ?? []);

	function choose(id: string) {
		chosen = id;
		persona = REGULAR_PERSONA;
		look = '';
		step = 2;
		queueMicrotask(() => grid?.querySelector<HTMLElement>('[data-cell]')?.focus());
	}
	function pick(picked: PickedCharacter) {
		onpick(picked);
		open = false;
	}

	/** arrow keys move the focus within a grid of [data-cell] buttons */
	const thumb = (c: RegistryCharacter, p?: string) => characterImage(c, { persona: p }, AUTHORED_VIEW, { isMissing: isKnownMissing });
</script>

<PickerSheet bind:open title={m['creator.characters_picker_title']()} testid="character-picker-sheet">
		{#if step === 1 || !character}
			<div class="sc-picker-filters">
				<!-- svelte-ignore a11y_autofocus -->
				<input type="search" bind:value={query} placeholder={m['creator.characters_search']()} autofocus data-testid="picker-search" />
				<select bind:value={toonFilter} aria-label={m['creator.characters_own_toon']()}>
					<option value="">{m['creator.characters_all_toons']()}</option>
					{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
				</select>
				<label class="check"><input type="checkbox" bind:checked={missingOnly} /> {m['creator.characters_has_missing']()}</label>
			</div>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div onkeydown={gridArrows} bind:this={grid}>
				{#each sections as section (section.id)}
					{#if section.title}<h4>{section.title}</h4>{/if}
					<div class="sc-cards">
						{#each section.list as c (c.id)}
							{@const src = thumb(c)}
							<button type="button" class="sc-card" data-control-exempt="picture card" data-cell onclick={() => choose(c.id)} data-testid="picker-character-{c.id}">
								<span class="sc-card-img" style:background={c.color || 'transparent'}>{#if src}<img src={asset(src)} onerror={() => imageExists(src)} alt="" loading="lazy" />{/if}</span>
								<span>{characterName(c, locale)}</span>
								<span class="hint">{c.own_toon ? (toons.find((t) => t.id === c.own_toon)?.name ?? c.own_toon) : '–'}</span>
								{#if personaIds(c).length > 1}
									<span class="chips">{#each personaIds(c) as p (p)}<span class="sc-chip is-tag">{p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(c, p)?.label, locale) || p}</span>{/each}</span>
								{/if}
							</button>
						{/each}
					</div>
				{:else}
					<p class="hint">{m['creator.characters_none_found']()}</p>
				{/each}
			</div>
		{:else}
			<header class="step2">
				<button type="button" class="ghost" onclick={() => (step = 1)} data-testid="picker-back">{m['creator.characters_picker_back']()}</button>
				<span>{characterName(character, locale)}</span>
				{#if !castIds.has(character.id) && season}<span class="sc-mark">{m['creator.characters_picker_will_add']({ season })}</span>{/if}
			</header>
			{#if mode === 'persona'}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="sc-cards" onkeydown={gridArrows} bind:this={grid}>
					{#each personaIds(character) as p (p)}
						{@const src = thumb(character, p)}
						<button type="button" class="sc-card" data-control-exempt="picture card" data-cell onclick={() => pick({ character: character.id, persona: p })} data-testid="picker-persona-{p}">
							<span class="sc-card-img">{#if src}<img src={asset(src)} onerror={() => imageExists(src)} alt="" loading="lazy" />{/if}</span>
							<span>{characterName(character, locale, p)}</span>
							{#if castPersonas.includes(p)}<span class="hint">{m['creator.characters_in_cast']()}</span>{/if}
						</button>
					{/each}
				</div>
			{:else}
				<Tabs
					variant="ruled"
					label={m['creator.characters_personas']()}
					value={currentPersona?.id}
					onchange={(p) => ((persona = p), (look = ''))}
					tabs={personaIds(character).map((p) => ({ id: p, label: p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(character, p)?.label, locale) || p, testid: `picker-persona-${p}` }))}
				>
					{#snippet tab(t)}{t.label}{#if season && !castPersonas.includes(t.id)}<span class="hint"> +</span>{/if}{/snippet}
				</Tabs>
				{#if currentPersona && currentPersona.looks.length > 1}
					<Tabs
						variant="sub"
						label={m['creator.characters_looks']()}
						value={lookId}
						onchange={(l) => (look = l)}
						tabs={currentPersona.looks.map((l) => ({ id: l.id, label: lookLabel(currentPersona, l.id, locale), testid: `picker-look-${l.id}` }))}
					/>
				{/if}
				<label class="check"><input type="checkbox" bind:checked={allMoods} /> {m['creator.characters_picker_all_moods']()}</label>
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="sc-cards" style:--cards-min="6.5rem" onkeydown={gridArrows} bind:this={grid}>
					{#each rowMoods as mood (mood)}
						{@const path = lookImage(currentLook, mood, AUTHORED_VIEW)}
						<button
							type="button"
							class="sc-card"
							data-control-exempt="picture cell"
							class:is-missing={!path}
							class:is-selected={value?.character === character.id && (value?.persona || REGULAR_PERSONA) === currentPersona?.id && (value?.look || defaultLookId(currentPersona)) === lookId && (value?.mood || DEFAULT_MOOD) === mood}
							data-cell
							onclick={() => pick({ character: character.id, persona: currentPersona?.id ?? REGULAR_PERSONA, look: lookId, mood })}
							title={path || m['creator.characters_missing_add']()}
							data-testid="picker-mood-{mood}"
						>
							<span class="sc-card-img">{#if path}<img src={asset(path)} alt="" loading="lazy" />{:else}<span class="hint">{m['creator.characters_missing_short']()}</span>{/if}</span>
							<span>{moodLabel(mood, locale, moods)}</span>
						</button>
					{/each}
				</div>
			{/if}
		{/if}
</PickerSheet>

<style>
	.check {
		display: flex;
		gap: 0.25rem;
		align-items: center;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	h4 {
		margin: 0.5rem 0 0.3rem;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.15rem;
	}
	.step2 {
		display: flex;
		gap: 0.6rem;
		align-items: center;
		flex-wrap: wrap;
	}
</style>
