<script lang="ts">
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { REGULAR_PERSONA, castSeasons, entities, findPersona, personaIds, pickText, toonStar, type LocaleText, type RegistryCharacter, type ToonCast } from '@scutoons/content';
	import { addToSeasonCast, castOf, edit, ensureCast, registryOf, type ToonInfo } from '$lib/editor/characters.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { moveAt } from '$lib/editor/move.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { places } from '$lib/shell/places.svelte.ts';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';

	/**
	 * Where a character is cast: every toon season it is in, with the personas it appears as there, its role per toon
	 * language and its place in the season's cast order; Add puts it into another season, ✕ takes it out (the toon's star
	 * stays in every season). The cast documents are saved with the page. The season's own view of its cast comes with
	 * the season sheet – TODO(#665).
	 */
	let { character, registry, casts, toons }: { character: RegistryCharacter; registry: ContentDoc; casts: Record<string, ContentDoc>; toons: ToonInfo[] } = $props();

	const nav = getCreatorNav();
	$effect(() => places.load(nav.toons));

	const id = $derived(character.id);
	const reg = $derived(registryOf(registry));
	const personas = $derived(personaIds(character));
	const personaLabel = (p: string) => (p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(character, p)?.label, textLanguage.names, []) || p);
	const toonName = (t: string) => toons.find((x) => x.id === t)?.name ?? t;

	interface Membership {
		toon: string;
		season: number;
		at: number;
		of: number;
		personas: string[];
		role?: LocaleText;
		star: boolean;
		languages: string[];
	}
	const memberships = $derived<Membership[]>(
		toons.flatMap((t) => {
			const cast = casts[t.id] ? castOf(casts[t.id]) : null;
			const star = toonStar(reg, t.id) === id;
			return castSeasons(cast).flatMap((season) => {
				const list = cast?.seasons[String(season)] ?? [];
				const at = list.findIndex((e) => e.character === id);
				if (at < 0) return [];
				const entry = list[at];
				return [{ toon: t.id, season, at, of: list.length, personas: entry.personas, role: entry.role, star, languages: t.languages.languages as string[] }];
			});
		})
	);
	/** the seasons it is not in yet: every season a toon has (its content, its cast, and season 1) */
	const open = $derived(
		toons.flatMap((t) => {
			const cast = casts[t.id] ? castOf(casts[t.id]) : null;
			const seasons = [...new Set([1, ...(places.parts[t.id]?.seasons ?? []), ...castSeasons(cast)])].sort((a, b) => a - b);
			return seasons.filter((s) => !memberships.some((x) => x.toon === t.id && x.season === s)).map((s) => ({ toon: t.id, season: s }));
		})
	);

	function change(toon: string, fn: (cast: ToonCast) => void, key?: string) {
		const doc = casts[toon];
		if (!doc) return;
		ensureCast(doc);
		edit<ToonCast>(doc, fn, key);
	}
	const entryOf = (f: ToonCast, season: number) => (f.seasons[String(season)] ?? []).find((e) => e.character === id);
	function togglePersona(x: Membership, persona: string, on: boolean) {
		change(x.toon, (f) => {
			const entry = entryOf(f, x.season);
			if (!entry) return;
			entry.personas = on ? [...new Set([...entry.personas, persona])] : entry.personas.filter((p) => p !== persona);
			if (!entry.personas.length) entry.personas = [REGULAR_PERSONA];
		});
	}
	function setRole(x: Membership, value: LocaleText) {
		change(
			x.toon,
			(f) => {
				const entry = entryOf(f, x.season);
				if (!entry) return;
				if (Object.values(value).some((t) => t?.trim())) entry.role = value;
				else delete entry.role;
			},
			`role-${x.season}-${id}`
		);
	}
	function move(x: Membership, by: number) {
		const to = x.at + by;
		if (to < 0 || to >= x.of) return;
		change(x.toon, (f) => moveAt(f.seasons[String(x.season)], x.at, to));
	}
	function remove(x: Membership) {
		if (x.star) return;
		change(x.toon, (f) => (f.seasons[String(x.season)] = (f.seasons[String(x.season)] ?? []).filter((e) => e.character !== id)));
	}
	function add(value: string) {
		const [toon, season] = value.split('\u0000');
		const doc = casts[toon];
		if (doc) addToSeasonCast(doc, Number(season), id);
	}
</script>

<div class="links" data-testid="character-links">
	{#each memberships as x (`${x.toon}/${x.season}`)}
		<div class="membership" data-testid="membership-{x.toon}-{x.season}">
			<div class="head">
				<a class="where" href="/{x.toon}/season/{x.season}">{toonName(x.toon)} · {fileLabel(entities.season(x.toon, x.season))}</a>
				{#if x.star}<span class="star" title={m['creator.cast_star_locked']()} data-testid="membership-star">{m['creator.cast_star']()}</span>{/if}
				<span class="order" title={m['creator.cast_order']()}>{x.at + 1} / {x.of}</span>
				<button type="button" class="ib" disabled={x.at === 0} onclick={() => move(x, -1)} aria-label={m['creator.cast_earlier']()} title={m['creator.cast_earlier']()}><Icon name="chevron-up" /></button>
				<button type="button" class="ib" disabled={x.at === x.of - 1} onclick={() => move(x, 1)} aria-label={m['creator.cast_later']()} title={m['creator.cast_later']()}><Icon name="chevron-down" /></button>
				<button type="button" class="ib" disabled={x.star} onclick={() => remove(x)} aria-label={x.star ? m['creator.cast_star_locked']() : m['creator.cast_leave']()} title={x.star ? m['creator.cast_star_locked']() : m['creator.cast_leave']()} data-testid="membership-remove"><CloseIcon /></button>
			</div>
			{#if personas.length > 1}
				<div class="personas" role="group" aria-label={m['creator.cast_personas']()}>
					{#each personas as p (p)}
						{@const on = x.personas.includes(p)}
						{@const last = on && x.personas.length === 1}
						<button type="button" class="chip" aria-pressed={on} disabled={last} title={last ? m['creator.cast_last_persona']() : undefined} onclick={() => togglePersona(x, p, !on)}>{personaLabel(p)}</button>
					{/each}
				</div>
			{/if}
			<LocaleMapField value={x.role} languages={x.languages} label={m['creator.cast_role']()} onchange={(v) => setRole(x, v)} testid="membership-role-{x.toon}-{x.season}" />
		</div>
	{/each}
	{#if open.length}
		<select class="add" value="" onchange={(e) => (add(e.currentTarget.value), (e.currentTarget.value = ''))} aria-label={m['creator.cast_join']()} data-testid="membership-add">
			<option value="" disabled>{m['creator.cast_join']()}</option>
			{#each open as o (`${o.toon}/${o.season}`)}
				<option value="{o.toon}{'\u0000'}{o.season}">{toonName(o.toon)} · {fileLabel(entities.season(o.toon, o.season))}</option>
			{/each}
		</select>
	{/if}
</div>

<style>
	.links {
		display: grid;
		gap: 0.75rem;
	}
	.membership {
		display: grid;
		gap: 0.375rem;
	}
	.membership + .membership {
		padding-top: 0.75rem;
		border-top: var(--editor-edge) dotted var(--editor-line);
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		min-width: 0;
	}
	.where {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--editor-text);
	}
	.star {
		padding: 0 0.4rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 999px;
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
	.order {
		color: var(--editor-muted);
		font-size: var(--editor-small);
		font-variant-numeric: tabular-nums;
	}
	.ib {
		all: unset;
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		border-radius: 50%;
		color: var(--editor-muted);
		cursor: pointer;
	}
	.ib:hover:not(:disabled),
	.ib:focus-visible {
		background: var(--editor-hover);
		color: var(--editor-text);
	}
	.ib:disabled {
		opacity: 0.35;
		cursor: default;
	}
	.personas {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.chip {
		height: calc(var(--control-height) - 0.25rem);
		padding: 0 0.6rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: none;
		color: var(--editor-muted);
		font: inherit;
		font-size: var(--editor-small);
		cursor: pointer;
	}
	.chip[aria-pressed='true'] {
		border-color: transparent;
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
	}
	.chip:disabled {
		cursor: default;
	}
	.add {
		width: 100%;
	}
</style>
