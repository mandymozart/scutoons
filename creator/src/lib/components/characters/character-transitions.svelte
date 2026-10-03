<script lang="ts">
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { conventionTransitionPath, defaultLookId, lookLabel, type CharacterPersona } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { editPersona } from '$lib/editor/characters.ts';
	import { cleanText } from '$lib/editor/character-form.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';

	/** A persona's transitions between two of its looks: image, from → to, label; a new one under the convention name. */
	let {
		registry,
		characterId,
		persona,
		languages,
		source
	}: { registry: ContentDoc; characterId: string; persona: CharacterPersona; languages: string[]; source: string } = $props();

	const lookIds = $derived(persona.looks.map((l) => l.id));
	function change(fn: (p: CharacterPersona) => void, key?: string) {
		editPersona(registry, characterId, persona.id, fn, key ? `${characterId}:${persona.id}:${key}` : undefined);
	}
	function add() {
		const from = lookIds.find((l) => l !== defaultLookId(persona)) ?? lookIds[0];
		const to = defaultLookId(persona);
		change((p) => {
			p.transitions ??= [];
			p.transitions.push({ from, to, image: conventionTransitionPath(characterId, persona.id, from, to) });
		});
	}
</script>

<div class="transitions" data-testid="transitions">
	<h3>{m['creator.characters_transitions']()}</h3>
	{#each persona.transitions ?? [] as t, i (i)}
		<div class="transition">
			<img src={asset(t.image)} alt="" loading="lazy" />
			<div class="col">
				<span class="inline">
					<select value={t.from} onchange={(e) => change((p) => p.transitions && (p.transitions[i].from = e.currentTarget.value))}>
						{#each lookIds as l (l)}<option value={l}>{lookLabel(persona, l, source)}</option>{/each}
					</select>
					→
					<select value={t.to} onchange={(e) => change((p) => p.transitions && (p.transitions[i].to = e.currentTarget.value))}>
						{#each lookIds as l (l)}<option value={l}>{lookLabel(persona, l, source)}</option>{/each}
					</select>
				</span>
				<input type="text" value={t.image} onchange={(e) => change((p) => p.transitions && (p.transitions[i].image = e.currentTarget.value.trim()))} />
				<LocaleMapField value={t.label} {languages} label={m['creator.characters_look_label']()} onchange={(v) => change((p) => { if (!p.transitions) return; const x = cleanText(v); if (x) p.transitions[i].label = x; else delete p.transitions[i].label; }, `transition-${i}`)} />
			</div>
			<button type="button" class="ghost" onclick={() => change((p) => { p.transitions?.splice(i, 1); if (!p.transitions?.length) delete p.transitions; })} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
		</div>
	{:else}
		<p class="hint">{m['creator.characters_no_transitions_short']()}</p>
	{/each}
	{#if lookIds.length > 1}
		<button type="button" class="ghost" onclick={add} data-testid="add-transition"><Icon name="plus" /> {m['creator.characters_add_transition']()}</button>
	{/if}
</div>

<style>
	h3 {
		margin: 0 0 0.4rem;
	}
	.transitions {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		align-items: flex-start;
	}
	.transition {
		display: grid;
		grid-template-columns: 6rem minmax(0, 1fr) auto;
		gap: 0.5rem;
		width: 100%;
	}
	.transition img {
		width: 6rem;
		height: 6rem;
		object-fit: cover;
		background: var(--surface-hover);
	}
	.col {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.inline {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		font-weight: 400;
	}
</style>
