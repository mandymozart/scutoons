<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import type { CreditPerson, RegistryCharacter } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { editCharacter, type ToonInfo } from '$lib/editor/characters.ts';
	import { cleanText, toHex } from '$lib/editor/character-form.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';

	/**
	 * A character's identity (character level): name and description per language of its toons, colour, testo, own
	 * toon, voice, tags, tendencies.
	 */
	let {
		registry,
		character,
		languages,
		toons,
		people
	}: { registry: ContentDoc; character: RegistryCharacter; languages: string[]; toons: ToonInfo[]; people: CreditPerson[] } = $props();

	const id = $derived(character.id);
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

<section class="block">
	<LocaleMapField value={character.name} {languages} label={m['creator.characters_name']()} required testid="character-name" onchange={(v) => change((c) => (c.name = v), 'name')} />
	<LocaleMapField value={character.description} {languages} label={m['creator.characters_description']()} multiline testid="character-description" onchange={(v) => setField('description', cleanText(v))} />
</section>

<section class="block">
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
			<select value={character.own_toon ?? ''} onchange={(e) => setField('own_toon', e.currentTarget.value)} data-testid="character-own-toon">
				<option value="">–</option>
				{#each toons as t (t.id)}<option value={t.id}>{t.name} · {t.id}</option>{/each}
			</select>
		</label>
		<label>
			{m['creator.characters_voiced_by']()}
			<select value={character.voiced_by ?? ''} onchange={(e) => setField('voiced_by', e.currentTarget.value)} data-testid="character-voiced-by">
				<option value="">–</option>
				{#each people.filter((p) => !p.merged_into).sort((a, b) => a.name.localeCompare(b.name)) as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
				{#if character.voiced_by && !people.some((p) => p.id === character.voiced_by)}<option value={character.voiced_by}>{character.voiced_by} (?)</option>{/if}
			</select>
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
	.block {
		border-top: 1px solid var(--border-subtle);
		padding-top: 0.6rem;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	/* the field grid's column width: 1/8 rem under the default, so the page gap (1rem) still fits as many columns as
	   the old 12rem + 0.75rem */
	.sc-fields {
		--fields-fill-min: 11.875rem;
	}
	label {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
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
	/* the native colour picker as the swatch (square, the design's 2px rule) */
	.picker {
		width: 2rem;
		height: 2rem;
		padding: 0;
		border: 2px solid var(--border-muted, #ccc);
		background: transparent;
		cursor: pointer;
		flex: none;
	}
</style>
