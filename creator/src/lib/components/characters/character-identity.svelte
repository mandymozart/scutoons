<script lang="ts">
	import { Select } from '@scutoons/ui/editor';
	import { CloseIcon } from '@scutoons/ui';
	import type { RegistryCharacter } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { editCharacter, type ToonInfo } from '$lib/editor/characters.ts';
	import { cleanText, toHex } from '$lib/editor/character-form.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import { mapPlaces } from '$lib/editor/map.ts';
	import { contentApi } from '$lib/api.ts';
	import { mapName } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';

	/**
	 * A character's identity (character level): name and description per language of its toons, colour, testo, own
	 * toon, home, tags, tendencies.
	 */
	let {
		registry,
		character,
		languages,
		toons
	}: { registry: ContentDoc; character: RegistryCharacter; languages: string[]; toons: ToonInfo[] } = $props();

	const id = $derived(character.id);
	const regionName = (r: { id: string; name: unknown }) => mapName(r.name as Record<string, string>, getLocale()) || r.id;
	// the avatar's home (agents/research/CAVE.md D1): a map place (a missing one is added in the map builder) and the home
	// scene – what the home looks like, set per avatar
	let places = $state<{ id: string; name: unknown }[]>([]);
	let scenes = $state<{ id: string; name: unknown }[]>([]);
	$effect(() => {
		void mapPlaces().then((list) => (places = [...list].sort((a, b) => regionName(a).localeCompare(regionName(b)))));
		void contentApi.catalogDocs('home-scenes').then((r) => (scenes = r.entries.filter((e) => !e.removed).map((e) => ({ id: e.id, name: e.content.name }))), () => {});
	});
	function change(fn: (c: RegistryCharacter) => void, key?: string) {
		editCharacter(registry, id, fn, key ? `${id}:${key}` : undefined);
	}
	/** set a field, removing it when empty */
	function setField<K extends keyof RegistryCharacter>(key: K, value: RegistryCharacter[K] | undefined | '') {
		change((c) => {
			if (value === undefined || value === '' || (Array.isArray(value) && !value.length)) delete c[key];
			else c[key] = value as RegistryCharacter[K];
		}, String(key));
	}
</script>

<section class="sc-section">
	<LocaleMapField value={character.name} {languages} label={m['creator.characters_name']()} required testid="character-name" onchange={(v) => change((c) => (c.name = v), 'name')} />
	<LocaleMapField value={character.description} {languages} label={m['creator.characters_description']()} multiline testid="character-description" onchange={(v) => setField('description', cleanText(v))} />
</section>

<section class="sc-section">
	<div class="sc-fields" data-cols="fill">
		<label>
			{m['creator.characters_color']()}
			<span class="inline">
				<input type="color" class="picker" value={toHex(character.color)} oninput={(e) => setField('color', e.currentTarget.value)} aria-label="{m['creator.characters_color']()} picker" data-testid="character-color-picker" />
				<input type="text" value={character.color ?? ''} oninput={(e) => setField('color', e.currentTarget.value.trim())} data-testid="character-color" />
			</span>
		</label>
		<!-- no male/female: a testo level 0–100 (owner 2026-10-01, ITEMS.md "Substances"); empty = not set -->
		<label>
			{m['creator.characters_testo']()} {character.testo ?? '–'}
			<span class="inline">
				<input type="range" min="0" max="100" step="1" value={character.testo ?? 50} oninput={(e) => setField('testo', Number(e.currentTarget.value))} data-testid="character-testo" />
				<button type="button" class="ghost" disabled={character.testo === undefined} onclick={() => setField('testo', undefined)} title={m['creator.characters_testo_clear']()} aria-label={m['creator.characters_testo_clear']()}><CloseIcon /></button>
			</span>
		</label>
		<label>
			{m['creator.characters_own_toon']()}
			<Select label={m['creator.characters_own_toon']()} value={character.own_toon ?? ''} options={[{ value: '', label: '–' }, ...toons.map((t) => ({ value: t.id, label: `${t.name} · ${t.id}` }))]} onchange={(v) => setField('own_toon', v)} data-testid="character-own-toon" />
		</label>
		<label>
			{m['creator.characters_home']()}
			<Select
				label={m['creator.characters_home']()}
				value={character.home ?? ''}
				options={[
					{ value: '', label: '–' },
					...places.map((p) => ({ value: p.id, label: regionName(p) })),
					...(character.home && !places.some((p) => p.id === character.home) ? [{ value: character.home, label: `${character.home} (?)` }] : [])
				]}
				onchange={(v) => setField('home', v)}
				data-testid="character-home"
			/>
		</label>
		<label>
			{m['creator.characters_home_scene']()}
			<Select
				label={m['creator.characters_home_scene']()}
				value={character.home_scene ?? ''}
				options={[
					{ value: '', label: m['creator.characters_home_scene_default']() },
					...scenes.map((s) => ({ value: s.id, label: regionName(s) })),
					...(character.home_scene && !scenes.some((s) => s.id === character.home_scene) ? [{ value: character.home_scene, label: `${character.home_scene} (?)` }] : [])
				]}
				onchange={(v) => setField('home_scene', v)}
				data-testid="character-home-scene"
			/>
		</label>
		<label>
			{m['creator.characters_tags']()}
			<input type="text" value={(character.tags ?? []).join(', ')} onchange={(e) => setField('tags', e.currentTarget.value.split(',').map((t) => t.trim()).filter(Boolean))} />
		</label>
		<label>
			{m['creator.characters_tendencies']()}
			<input type="text" value={(character.tendencies ?? []).join(', ')} placeholder="psychedelics" onchange={(e) => setField('tendencies', e.currentTarget.value.split(',').map((t) => t.trim().toLowerCase()).filter(Boolean))} data-testid="character-tendencies" />
		</label>
	</div>
</section>

<style>
	/* the field grid's column width: 1/8 rem under the default, so the page gap (1rem) still fits as many columns as
	   the old 12rem + 0.75rem */
	.sc-fields {
		--fields-fill-min: 11.875rem;
	}
	label {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--tool-text);
		font-weight: var(--font-weight-regular);
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.inline {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		font-weight: 400;
	}
	.inline > input:not(.picker) {
		flex: 1;
		min-width: 0;
	}
	/* the native colour picker as the swatch (square, the design's 2px rule) */
	.picker {
		width: 2rem;
		height: 2rem;
		padding: 0;
		border: 2px solid var(--tool-line);
		background: transparent;
		cursor: pointer;
		flex: none;
	}
</style>
