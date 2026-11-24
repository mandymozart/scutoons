<script lang="ts">
	import { Select } from '@scutoons/ui/editor';
	import type { CharacterPersona, RegistryCharacter } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { CharacterModel } from '$lib/editor/character-model.svelte.ts';
	import { editPersona } from '$lib/editor/characters.ts';
	import { idleProblems } from '$lib/editor/core-model.ts';
	import { keepSkeleton, readSkeleton } from '$lib/editor/asset-skeletons.ts';
	import { idleChoices, idleOf, idleValue, type IdleChoice, type LibraryAnimation } from '$lib/editor/persona-idle.ts';
	import { assetsApi } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A persona's idle (#817): what its model plays when it stands – its own first clip (none), a clip of its model by
	 * name, or a shared animation of the library. Only what the core says plays on the model is offered.
	 */
	let { registry, character, persona, model }: { registry: ContentDoc; character: RegistryCharacter; persona: CharacterPersona; model: CharacterModel } = $props();

	let animations = $state<LibraryAnimation[]>([]);
	let clips = $state<IdleChoice[]>([]);
	let library = $state<IdleChoice[]>([]);
	const value = $derived(idleValue(persona.animations?.idle));

	$effect(() => {
		void (async () => {
			const list = await assetsApi.list({ kind: 'animation', page_size: 200 }).catch(() => null);
			const found: LibraryAnimation[] = [];
			for (const a of list?.items ?? []) found.push({ path: a.path, title: a.title, skeleton: a.skeleton ?? (await readSkeleton(a.path).catch(() => null)) ?? undefined });
			animations = found;
		})();
	});
	$effect(() => {
		const facts = model.facts;
		if (facts && model.path) void keepSkeleton(model.path, facts.skeleton).catch(() => undefined);
	});
	$effect(() => {
		const skeleton = model.facts?.skeleton;
		const offered = animations;
		if (!skeleton) {
			clips = [];
			library = [];
			return;
		}
		void idleChoices(skeleton, offered, idleProblems).then((c) => {
			clips = c.clips;
			library = c.library;
		});
	});

	function pick(next: string) {
		const idle = idleOf(next);
		editPersona(
			registry,
			character.id,
			persona.id,
			(p) => {
				if (idle) p.animations = { ...p.animations, idle };
				else if (p.animations) {
					delete p.animations.idle;
					if (!Object.keys(p.animations).length) delete p.animations;
				}
			},
			`${character.id}:${persona.id}:idle`
		);
	}
</script>

<label class="idle" title={!model.facts ? m['creator.characters_idle_no_model']() : !library.length ? m['creator.characters_idle_none_fit']() : undefined}>
	<span>{m['creator.characters_idle']()}</span>
	{#if model.facts}
		<Select
			label={m['creator.characters_idle']()}
			{value}
			options={[
				{ value: '', label: m['creator.characters_idle_own']() },
				...(value && ![...clips, ...library].some((c) => `${c.kind}:${c.value}` === value) ? [{ value, label: value.replace(/^\w+:/, '') }] : []),
				...clips.map((c) => ({ value: `clip:${c.value}`, label: c.label, group: m['creator.characters_idle_clips']() })),
				...library.map((c) => ({ value: `animation:${c.value}`, label: c.label, group: m['creator.characters_idle_library']() }))
			]}
			onchange={(v) => pick(v)}
			data-testid="persona-idle"
		/>
	{/if}
</label>

<style>
	.idle {
		display: flex;
		flex-direction: column;
		gap: var(--space-025, 0.25rem);
	}
</style>
