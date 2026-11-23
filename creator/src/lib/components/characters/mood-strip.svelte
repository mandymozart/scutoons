<script lang="ts">
	import { ToolGroup, type ToolGroupOption } from '@scutoons/ui/editor';
	import { characterImage, lookMoods, lookPortraitMoods, moodLabel, type CharacterLook, type CharacterView, type MoodVocabulary, type RegistryCharacter } from '@scutoons/content';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { edgeFade } from '$lib/editor/edge-fade.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The head of the character's Expressions: the look's moods as a strip (a click shows one on the Sheet) and the view
	 * the Sheet shows (face · bust · figure).
	 */
	let {
		character,
		persona,
		look,
		moods,
		locale,
		mood = $bindable('neutral'),
		view = $bindable<CharacterView>('portrait')
	}: {
		character: RegistryCharacter;
		persona: string;
		look: CharacterLook;
		moods: MoodVocabulary;
		locale: string;
		mood?: string;
		view?: CharacterView;
	} = $props();

	const VIEWS: ToolGroupOption[] = $derived([
		{ value: 'avatar', icon: 'view-avatar', name: m['creator.characters_view_avatar'](), label: m['creator.characters_view_avatar']() },
		{ value: 'portrait', icon: 'view-portrait', name: m['creator.characters_view_portrait'](), label: m['creator.characters_view_portrait']() },
		{ value: 'full', icon: 'view-full', name: m['creator.characters_view_full'](), label: m['creator.characters_view_full']() }
	]);
	const strip = $derived(lookMoods(look, moods));
	const filled = $derived(lookPortraitMoods(look).length);
	const thumb = (x: string) => characterImage(character, { persona, look: look.id, mood: x }, 'avatar', { isMissing: isKnownMissing });
	const choose = (value: string) => (view = value as CharacterView);
</script>

<div class="moods" data-testid="mood-panel">
	<div class="head">
		<ToolGroup label={m['creator.characters_view']()} options={VIEWS} value={view} expanded onchoose={choose} />
		<span class="count" title={m['creator.characters_moods_filled']()}>{filled} / {strip.length}</span>
	</div>
	<div class="strip" use:edgeFade role="listbox" aria-label={m['creator.characters_moods']()} data-testid="mood-strip">
		{#each strip as x (x)}
			<button type="button" class="mood" class:on={x === mood} role="option" aria-selected={x === mood} onclick={() => (mood = x)} data-control-exempt="a mood thumbnail with its word" data-testid="strip-{x}">
				<span class="pic">{#if thumb(x)}<img src={asset(thumb(x))} alt="" loading="lazy" draggable="false" />{/if}</span>
				<span class="word">{moodLabel(x, locale, moods)}</span>
			</button>
		{/each}
	</div>
</div>

<style>
	.moods {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		min-width: 0;
		padding: 0 var(--editor-pad) var(--editor-pad);
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.count {
		margin-inline-start: auto;
		color: var(--tool-muted);
		font-size: var(--editor-small);
		font-variant-numeric: tabular-nums;
	}
	.strip {
		display: flex;
		gap: 0.375rem;
		overflow-x: auto;
		scrollbar-width: none;
		--fade: 2.5rem;
	}
	.strip::-webkit-scrollbar {
		display: none;
	}
	.strip:global([data-fade='r']) {
		mask-image: linear-gradient(to right, #000 calc(100% - var(--fade)), transparent);
	}
	.strip:global([data-fade='l']) {
		mask-image: linear-gradient(to right, transparent, #000 var(--fade));
	}
	.strip:global([data-fade='lr']) {
		mask-image: linear-gradient(to right, transparent, #000 var(--fade), #000 calc(100% - var(--fade)), transparent);
	}
	.mood {
		all: unset;
		box-sizing: border-box;
		flex: none;
		width: 4.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		cursor: pointer;
	}
	.pic {
		display: block;
		aspect-ratio: 1;
		border-radius: var(--editor-radius);
		background: var(--tool-control);
		overflow: hidden;
	}
	.pic img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.mood.on .pic {
		outline: 2px solid var(--tool-text);
		outline-offset: -2px;
	}
	.mood:focus-visible .pic {
		outline: 2px solid var(--tool-text);
	}
	.word {
		color: var(--tool-muted);
		font-size: var(--editor-small);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.mood.on .word {
		color: var(--tool-text);
	}
</style>
