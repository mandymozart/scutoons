<script lang="ts">
	// The mood chip on the stage (#282): the character's face for the mood it plays now and its name; a click opens the
	// moods as faces (the character's portraits) – auto lets the engine's ladder choose. The mood is the first of the
	// character's properties the stage will simulate (look, persona … follow as more chips).
	import { clickOutside } from '@scutoons/ui';
	import { Button, Tile } from '@scutoons/ui/editor';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	let {
		moods,
		portrait,
		playing,
		value,
		onpick
	}: {
		moods: string[];
		/** the portrait's CDN path a mood shows – the deck's core picks it (#367), '' = none */
		portrait: (mood: string) => string;
		/** the mood the engine plays now (the ladder's, in auto) */
		playing: string;
		/** 'auto' or a mood asked for */
		value: string;
		onpick: (mood: string) => void;
	} = $props();

	let open = $state(false);
	const shown = $derived(value === 'auto' ? playing : value);
	const face = (mood: string) => {
		const path = portrait(mood);
		return path ? asset(path) : '';
	};
	function pick(mood: string) {
		open = false;
		onpick(mood);
	}
</script>

<div class="wrap" {@attach clickOutside(() => (open = false))}>
	<Button aria-haspopup="listbox" aria-expanded={open} title={m['creator.sound_mood']()} onclick={() => (open = !open)} data-testid="sound-mood">
		{#if face(shown)}<img class="face" src={face(shown)} alt="" />{/if}
		<span class="k">{m['creator.sound_mood']()}</span>
		<span>{value === 'auto' ? m['creator.sound_mood_auto']() : value}</span>
	</Button>
	{#if open}
		<ul class="faces" role="listbox" aria-label={m['creator.sound_mood']()}>
			{#each ['auto', ...moods] as mood (mood)}
				<li>
					<Tile label={mood === 'auto' ? m['creator.sound_mood_auto']() : mood} on={value === mood} role="option" aria-selected={value === mood} onclick={() => pick(mood)} data-testid="sound-mood-{mood}">
						{#snippet picture()}
							{#if mood !== 'auto' && face(mood)}<img src={face(mood)} alt="" />{:else if mood === 'auto' && face(playing)}<img class="auto" src={face(playing)} alt="" />{/if}
						{/snippet}
					</Tile>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.wrap {
		position: relative;
	}
	.face {
		width: 1.25rem;
		height: 1.25rem;
		border-radius: 50%;
		object-fit: cover;
	}
	.k {
		color: var(--tool-muted);
		margin-right: var(--space-025);
	}
	.faces {
		position: absolute;
		z-index: 5;
		left: 0;
		bottom: calc(100% + 0.25rem);
		margin: 0;
		padding: 0.375rem;
		list-style: none;
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		width: min(22rem, 70vw);
		box-sizing: border-box;
		gap: 0.25rem;
		background: var(--tool-popover);
		text-shadow: var(--tool-halo);
		color: var(--tool-text);
		border: 1px solid var(--tool-line);
		border-radius: var(--editor-radius);
		max-height: 18rem;
		overflow-y: auto;
	}
	.faces img {
		width: 2.5rem;
		height: 2.5rem;
		object-fit: cover;
	}
	.faces img.auto {
		opacity: 0.55;
	}
</style>
