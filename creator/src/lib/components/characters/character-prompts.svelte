<script lang="ts">
	import {
		AUTHORED_VIEW,
		characterBaseViewsBrief,
		characterMoodsBrief,
		findPersona,
		lookImage,
		moodLabel,
		wantedImages,
		type AssetRequests,
		type CharacterRegistry,
		type MoodVocabulary,
		type RegistryCharacter
	} from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Prompts" (end of the character page): ready-to-send briefs for another AI (an image generator) or a human artist,
	 * filled in for the persona + look shown – the base views (workflows/characters/base-views.md) and the look's moods
	 * still without a portrait. Copy, attach the image named in the first line, send. Replaces the "Actions" placeholder
	 * (agents/AUTOMATION.md "Creator roadmap") until those actions exist.
	 */
	let {
		registry,
		character,
		persona,
		look,
		moods,
		requests,
		locale
	}: {
		registry: CharacterRegistry | null;
		character: RegistryCharacter;
		persona: string;
		look: string;
		moods: MoodVocabulary;
		requests: AssetRequests | null;
		locale: string;
	} = $props();

	let copied = $state('');
	let fallback = $state<{ id: string; text: string } | null>(null);

	/** the look's moods without a portrait: the wanted ones (standard set + requests), then those added without an image */
	const missingMoods = $derived.by(() => {
		const current = findPersona(character, persona)?.looks.find((l) => l.id === look);
		const wanted = wantedImages(registry, { moods, requests, character: character.id })
			.filter((w) => w.persona === persona && w.look === look && w.view === AUTHORED_VIEW)
			.map((w) => w.mood);
		const added = Object.keys(current?.images ?? {}).filter((x) => !lookImage(current, x, AUTHORED_VIEW));
		return [...new Set([...wanted, ...added])].map((id) => ({ id, label: moodLabel(id, 'en', moods) }));
	});
	const prompts = $derived([
		{ id: 'base-views', title: m['creator.prompts_base_views'](), text: characterBaseViewsBrief(character, persona, look, locale) },
		...(missingMoods.length ? [{ id: 'moods', title: m['creator.prompts_moods']({ count: missingMoods.length }), text: characterMoodsBrief(character, persona, look, missingMoods, locale) }] : [])
	]);

	async function copy(id: string, text: string) {
		try {
			await navigator.clipboard.writeText(text);
			copied = id;
			fallback = null;
			setTimeout(() => (copied = ''), 2000);
		} catch {
			// no clipboard permission: the text is selectable below
			fallback = { id, text };
		}
	}
</script>

<div class="prompts" data-testid="character-prompts">
	<p class="hint">{m['creator.prompts_hint']()}</p>
	{#each prompts as p (p.id)}
		<details data-testid="prompt-{p.id}">
			<summary>{p.title}</summary>
			<button type="button" class="secondary" onclick={() => copy(p.id, p.text)} data-testid="copy-prompt-{p.id}">{copied === p.id ? '✓' : ''} {m['creator.prompts_copy']()}</button>
			<pre>{p.text}</pre>
			{#if fallback?.id === p.id}<textarea rows="8" readonly value={fallback.text}></textarea>{/if}
		</details>
	{/each}
</div>

<style>
	.prompts {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.prompts > .hint {
		margin: 0;
	}
	details > button {
		margin-top: 0.5rem;
	}
	pre {
		white-space: pre-wrap;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		margin: 0.5rem 0 0;
	}
	textarea {
		width: 100%;
	}
</style>
