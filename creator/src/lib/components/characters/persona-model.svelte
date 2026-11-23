<script lang="ts">
	import { PERSONA_BODIES, personaModelSource, type CharacterPersona, type PersonaBody, type RegistryCharacter } from '@scutoons/content';
	import { FileSlot, ModelPreview } from '@scutoons/media';
	import { Button } from '@scutoons/ui/editor';
	import { VIEWER_URL } from '$lib/config.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { editPersona } from '$lib/editor/characters.ts';
	import type { CharacterModel } from '$lib/editor/character-model.svelte.ts';
	import PersonaIdleField from '$lib/components/characters/persona-idle-field.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A persona's 3D model (owner 2026-10-13/14): the live preview of what the shown look uses – the look's model, the
	 * persona's, or the mannequin of its body preset (dashed, "Placeholder") –, the persona's model slot (a .glb from the
	 * asset manager), "Open in the deck" (a plain link to the deck until it opens on this rig – TODO(#859)), its idle (#817)
	 * and the body preset for the fallback: two legs / more legs / wings.
	 */
	let {
		registry,
		character,
		persona,
		lookId,
		model
	}: {
		registry: ContentDoc;
		character: RegistryCharacter;
		persona: CharacterPersona;
		lookId: string;
		model?: CharacterModel;
	} = $props();

	const source = $derived(personaModelSource(character, persona.id, lookId));
	const BODY_LABELS: Record<PersonaBody, () => string> = {
		'two-legs': () => m['creator.characters_body_two_legs'](),
		'more-legs': () => m['creator.characters_body_more_legs'](),
		wings: () => m['creator.characters_body_wings']()
	};

	function change(fn: (p: CharacterPersona) => void, key: string) {
		editPersona(registry, character.id, persona.id, fn, `${character.id}:${persona.id}:${key}`);
	}
	function setModel(path: string | null) {
		change((p) => (path ? (p.model = path) : delete p.model), 'model');
	}
	function setBody(body: PersonaBody) {
		change((p) => (body === 'two-legs' ? delete p.body : (p.body = body)), 'body');
	}
</script>

<div class="persona-model" data-testid="persona-model-block">
	<ModelPreview path={source.path} fallback={source.fallback} testid="persona-model-preview" />
	<div class="fields">
		<FileSlot kind="model" value={persona.model} label={m['creator.characters_model']()} place="characters/{character.id}/" player={false} onchange={setModel} testid="persona-model" />
		<span><Button icon="external-link" href="{VIEWER_URL}/live/deck" target="_blank" rel="noopener" data-testid="persona-open-deck">{m['character.open_in_deck']()}</Button></span>
		{#if source.fallback}<span class="hint" data-testid="persona-model-mannequin">{m['creator.characters_model_mannequin']()}</span>{/if}
		{#if model}<PersonaIdleField {registry} {character} {persona} {model} />{/if}
		<fieldset>
			<legend>{m['creator.characters_body']()}</legend>
			<span class="sc-seg" role="group" aria-label={m['creator.characters_body']()}>
				{#each PERSONA_BODIES as body (body)}
					<button type="button" aria-pressed={(persona.body ?? 'two-legs') === body} onclick={() => setBody(body)} data-testid="persona-body-{body}">{BODY_LABELS[body]()}</button>
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
		color: var(--tool-muted);
	}
	fieldset {
		margin: 0;
	}
</style>
