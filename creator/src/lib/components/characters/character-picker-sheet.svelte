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
	import { isKnownMissing, imageExists } from '$lib/editor/characters.ts';
	import { Sheet } from '@scutoons/ui';
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
	function arrows(event: KeyboardEvent) {
		const keys: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -100, ArrowDown: 100 };
		if (!(event.key in keys)) return;
		const container = event.currentTarget as HTMLElement;
		const cells = [...container.querySelectorAll<HTMLElement>('[data-cell]')];
		const index = cells.indexOf(document.activeElement as HTMLElement);
		if (index < 0) return;
		event.preventDefault();
		const columns = Math.max(1, getComputedStyle(cells[index].parentElement!).gridTemplateColumns.split(' ').length);
		const step = keys[event.key] === -100 ? -columns : keys[event.key] === 100 ? columns : keys[event.key];
		cells[Math.min(cells.length - 1, Math.max(0, index + step))]?.focus();
	}
	const thumb = (c: RegistryCharacter, p?: string) => characterImage(c, { persona: p }, AUTHORED_VIEW, { isMissing: isKnownMissing });
</script>

<Sheet bind:open title={m['creator.characters_picker_title']()} side="right" closeLabel={m['creator.characters_close']()}>
	<div class="picker" data-testid="character-picker-sheet">
		{#if step === 1 || !character}
			<div class="filters">
				<!-- svelte-ignore a11y_autofocus -->
				<input type="search" bind:value={query} placeholder={m['creator.characters_search']()} autofocus data-testid="picker-search" />
				<select bind:value={toonFilter} aria-label={m['creator.characters_own_toon']()}>
					<option value="">{m['creator.characters_all_toons']()}</option>
					{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
				</select>
				<label class="check"><input type="checkbox" bind:checked={missingOnly} /> {m['creator.characters_has_missing']()}</label>
			</div>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div onkeydown={arrows} bind:this={grid}>
				{#each sections as section (section.id)}
					{#if section.title}<h4>{section.title}</h4>{/if}
					<div class="cards">
						{#each section.list as c (c.id)}
							{@const src = thumb(c)}
							<button type="button" class="card" data-cell onclick={() => choose(c.id)} data-testid="picker-character-{c.id}">
								<span class="img" style:background={c.color || 'transparent'}>{#if src}<img src={asset(src)} onerror={() => imageExists(src)} alt="" loading="lazy" />{/if}</span>
								<strong>{characterName(c, locale)}</strong>
								<span class="hint">{c.own_toon ? (toons.find((t) => t.id === c.own_toon)?.name ?? c.own_toon) : '–'}</span>
								{#if personaIds(c).length > 1}
									<span class="chips">{#each personaIds(c) as p (p)}<span class="chip">{p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(c, p)?.label, locale) || p}</span>{/each}</span>
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
				<button type="button" class="ghost" onclick={() => (step = 1)} data-testid="picker-back">← {m['creator.characters_picker_back']()}</button>
				<strong>{characterName(character, locale)}</strong>
				{#if !castIds.has(character.id) && season}<span class="note">{m['creator.characters_picker_will_add']({ season })}</span>{/if}
			</header>
			{#if mode === 'persona'}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="cards" onkeydown={arrows} bind:this={grid}>
					{#each personaIds(character) as p (p)}
						{@const src = thumb(character, p)}
						<button type="button" class="card" data-cell onclick={() => pick({ character: character.id, persona: p })} data-testid="picker-persona-{p}">
							<span class="img">{#if src}<img src={asset(src)} onerror={() => imageExists(src)} alt="" loading="lazy" />{/if}</span>
							<strong>{characterName(character, locale, p)}</strong>
							{#if castPersonas.includes(p)}<span class="hint">{m['creator.characters_in_cast']()}</span>{/if}
						</button>
					{/each}
				</div>
			{:else}
				<nav class="tabs">
					{#each personaIds(character) as p (p)}
						<button type="button" class="tab" class:active={p === currentPersona?.id} onclick={() => ((persona = p), (look = ''))} data-testid="picker-persona-{p}">
							{p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(character, p)?.label, locale) || p}
							{#if season && !castPersonas.includes(p)}<span class="hint"> +</span>{/if}
						</button>
					{/each}
				</nav>
				{#if currentPersona && currentPersona.looks.length > 1}
					<nav class="tabs sub">
						{#each currentPersona.looks as l (l.id)}
							<button type="button" class="tab" class:active={l.id === lookId} onclick={() => (look = l.id)} data-testid="picker-look-{l.id}">{lookLabel(currentPersona, l.id, locale)}</button>
						{/each}
					</nav>
				{/if}
				<label class="check"><input type="checkbox" bind:checked={allMoods} /> {m['creator.characters_picker_all_moods']()}</label>
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="moods" onkeydown={arrows} bind:this={grid}>
					{#each rowMoods as mood (mood)}
						{@const path = lookImage(currentLook, mood, AUTHORED_VIEW)}
						<button
							type="button"
							class="cell"
							class:missing={!path}
							class:selected={value?.character === character.id && (value?.persona || REGULAR_PERSONA) === currentPersona?.id && (value?.look || defaultLookId(currentPersona)) === lookId && (value?.mood || DEFAULT_MOOD) === mood}
							data-cell
							onclick={() => pick({ character: character.id, persona: currentPersona?.id ?? REGULAR_PERSONA, look: lookId, mood })}
							title={path || m['creator.characters_missing_add']()}
							data-testid="picker-mood-{mood}"
						>
							<span class="img">{#if path}<img src={asset(path)} alt="" loading="lazy" />{:else}<span class="hint">{m['creator.characters_missing_short']()}</span>{/if}</span>
							<span>{moodLabel(mood, locale, moods)}</span>
						</button>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</Sheet>

<style>
	.picker {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		/* fill the sheet, never wider: the lists wrap into rows instead of scrolling sideways */
		width: 100%;
		max-width: 46rem;
		min-width: 0;
		overflow-x: hidden;
		font-size: 12px;
	}
	.filters {
		display: grid;
		grid-template-columns: 2fr 1fr 1fr auto;
		gap: 0.3rem;
		align-items: center;
	}
	@media (max-width: 700px) {
		.filters {
			grid-template-columns: 1fr 1fr;
		}
	}
	.check {
		display: flex;
		gap: 0.25rem;
		align-items: center;
		font-size: 11px;
	}
	h4 {
		margin: 0.5rem 0 0.3rem;
		font-size: 12px;
	}
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr));
		gap: 0.4rem;
	}
	.moods {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(6.5rem, 1fr));
		gap: 0.3rem;
	}
	.card,
	.cell {
		all: unset;
		box-sizing: border-box;
		min-width: 0;
		overflow-wrap: anywhere;
		cursor: pointer;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		padding: 0.3rem;
		border: 1px solid var(--border-subtle);
		background: var(--paper);
	}
	.card:hover,
	.cell:hover {
		border-color: var(--ink);
	}
	.card:focus-visible,
	.cell:focus-visible {
		outline: 2px solid var(--ink);
	}
	.cell.missing {
		border-style: dashed;
		border-color: var(--ink);
	}
	.cell.selected {
		border: 2px solid var(--ink);
		background: var(--primary-cyan);
	}
	.img {
		aspect-ratio: 1;
		width: 100%;
		background: var(--surface-hover);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.img img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.15rem;
	}
	.chip {
		font-size: 10px;
		border: 1px solid var(--ink);
		padding: 0 0.25rem;
	}
	.step2 {
		display: flex;
		gap: 0.6rem;
		align-items: center;
		flex-wrap: wrap;
	}
	.note {
		background: var(--primary-cyan);
		padding: 0 0.3rem;
		font-size: 11px;
	}
	.tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		border-bottom: 2px solid var(--ink);
		padding-bottom: 0.25rem;
	}
	.tabs.sub {
		border-bottom-width: 1px;
	}
	.tab {
		background: var(--paper);
		color: var(--ink);
		border: 2px solid var(--border-muted);
		padding: 0.2rem 0.5rem;
		font-size: 12px;
	}
	.tab.active {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
</style>
