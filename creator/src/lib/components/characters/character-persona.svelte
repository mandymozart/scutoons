<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import type { CharacterPersona, RegistryCharacter } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { editPersona, type ToonInfo } from '$lib/editor/characters.ts';
	import { cleanText, setPoint, toHex } from '$lib/editor/character-form.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';

	/**
	 * A persona's own fields (not the regular one): label, where it is introduced (toon / season / chapter), its colour and
	 * testo (the character's when empty), description, remove.
	 */
	let {
		registry,
		character,
		persona,
		languages,
		toons,
		onremove
	}: {
		registry: ContentDoc;
		character: RegistryCharacter;
		persona: CharacterPersona;
		languages: string[];
		toons: ToonInfo[];
		onremove: () => void;
	} = $props();

	function change(fn: (p: CharacterPersona) => void, key?: string) {
		editPersona(registry, character.id, persona.id, fn, key ? `${character.id}:${persona.id}:${key}` : undefined);
	}
</script>

<div class="sc-fields" data-cols="fit">
	<LocaleMapField value={persona.label} {languages} label={m['creator.characters_persona_label']()} required testid="persona-label-text" onchange={(v) => change((p) => (p.label = v), 'label')} />
	<div class="sc-fields intro" data-cols="fill">
		<label>
			{m['creator.characters_introduced']()}
			<select value={persona.introduced?.toon ?? ''} onchange={(e) => change((p) => { const x = setPoint(p.introduced, 'toon', e.currentTarget.value); if (x) p.introduced = x; else delete p.introduced; })}>
				<option value="">{m['creator.characters_always']()}</option>
				{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
			</select>
		</label>
		{#each ['season', 'chapter'] as const as key (key)}
			<label>
				{key === 'season' ? m['creator.characters_season']() : m['creator.characters_chapter']()}
				<input type="number" min="1" disabled={!persona.introduced} value={persona.introduced?.[key] ?? ''} onchange={(e) => change((p) => { const x = setPoint(p.introduced, key, e.currentTarget.value); if (x) p.introduced = x; })} />
			</label>
		{/each}
	</div>
</div>
<label class="persona-color">
	{m['creator.characters_persona_color']()}
	<span class="inline">
		<input type="color" class="picker" value={toHex(persona.color || character.color)} oninput={(e) => { const v = e.currentTarget.value; change((p) => { p.color = v; }, 'color'); }} aria-label="{m['creator.characters_persona_color']()} picker" data-testid="persona-color-picker" />
		<input type="text" value={persona.color ?? ''} placeholder={character.color} oninput={(e) => change((p) => { const v = e.currentTarget.value.trim(); if (v) p.color = v; else delete p.color; }, 'color')} data-testid="persona-color" />
	</span>
</label>
<label class="persona-color">
	{m['creator.characters_persona_testo']()} {persona.testo ?? character.testo ?? '–'}
	<span class="inline">
		<input type="range" min="0" max="100" step="1" value={persona.testo ?? character.testo ?? 50} oninput={(e) => { const v = Number(e.currentTarget.value); change((p) => { p.testo = v; }, 'testo'); }} data-testid="persona-testo" />
		<button type="button" class="ghost" disabled={persona.testo === undefined} onclick={() => change((p) => { delete p.testo; }, 'testo')} title={m['creator.characters_testo_clear']()} aria-label={m['creator.characters_testo_clear']()}><CloseIcon /></button>
	</span>
</label>
<LocaleMapField value={persona.description} {languages} label={m['creator.characters_description']()} multiline onchange={(v) => change((p) => { const t = cleanText(v); if (t) p.description = t; else delete p.description; }, 'description')} />
<button type="button" class="ghost danger" onclick={onremove}>{m['creator.characters_remove_persona']()}</button>

<style>
	/* the field grids' column widths: 1/8 rem under the defaults, so the page gap (1rem) still fits as many columns as the
	   old 12rem / 18rem + 0.75rem */
	.sc-fields[data-cols='fill'] {
		--fields-fill-min: 11.875rem;
	}
	.sc-fields[data-cols='fit'] {
		--fields-fit-min: 17.875rem;
	}
	.intro {
		align-self: end;
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
	.persona-color {
		max-width: 16rem;
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
	.danger {
		color: var(--error);
		align-self: flex-start;
	}
</style>
