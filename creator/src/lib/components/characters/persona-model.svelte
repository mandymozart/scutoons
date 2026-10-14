<script lang="ts">
	import { PERSONA_LEGS, personaModelSource, type CharacterPersona, type PersonaLegs, type RegistryCharacter } from '@scutoons/content';
	import { FileSlot, ModelPreview } from '@scutoons/media';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { editPersona } from '$lib/editor/characters.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A persona's 3D model (owner 2026-10-13/14): the live preview of what the shown look uses – the look's model, the
	 * persona's, or the mannequin of its body preset (dashed, "Placeholder") –, the persona's model slot (a .glb from the
	 * asset manager) and the body preset for the fallback: two legs / more legs.
	 */
	let {
		registry,
		character,
		persona,
		lookId
	}: {
		registry: ContentDoc;
		character: RegistryCharacter;
		persona: CharacterPersona;
		lookId: string;
	} = $props();

	const source = $derived(personaModelSource(character, persona.id, lookId));
	const LEG_LABELS: Record<PersonaLegs, () => string> = { two: () => m['creator.characters_legs_two'](), more: () => m['creator.characters_legs_more']() };

	function change(fn: (p: CharacterPersona) => void, key: string) {
		editPersona(registry, character.id, persona.id, fn, `${character.id}:${persona.id}:${key}`);
	}
	function setModel(path: string | null) {
		change((p) => (path ? (p.model = path) : delete p.model), 'model');
	}
	function setLegs(legs: PersonaLegs) {
		change((p) => (legs === 'two' ? delete p.legs : (p.legs = legs)), 'legs');
	}
</script>

<div class="persona-model" data-testid="persona-model-block">
	<ModelPreview path={source.path} fallback={source.fallback} testid="persona-model-preview" />
	<div class="fields">
		<FileSlot kind="model" value={persona.model} label={m['creator.characters_model']()} place="characters/{character.id}/" player={false} onchange={setModel} testid="persona-model" />
		{#if source.fallback}<span class="hint" data-testid="persona-model-mannequin">{m['creator.characters_model_mannequin']()}</span>{/if}
		<fieldset>
			<legend>{m['creator.characters_legs']()}</legend>
			<span class="sc-seg" role="group" aria-label={m['creator.characters_legs']()}>
				{#each PERSONA_LEGS as legs (legs)}
					<button type="button" aria-pressed={(persona.legs ?? 'two') === legs} onclick={() => setLegs(legs)} data-testid="persona-legs-{legs}">{LEG_LABELS[legs]()}</button>
				{/each}
			</span>
		</fieldset>
	</div>
</div>

<style>
	.persona-model {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		align-items: flex-start;
	}
	.fields {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		min-width: 0;
		max-width: 24rem;
	}
	.hint {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--muted);
	}
	fieldset {
		margin: 0;
	}
</style>
