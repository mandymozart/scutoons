<script lang="ts">
	// The mood chip on the stage (#282): the character's face for the mood it plays now and its name; a click opens the
	// moods as faces (the character's portraits) – auto lets the engine's ladder choose. The mood is the first of the
	// character's properties the stage will simulate (look, persona … follow as more chips).
	import { clickOutside } from '@scutoons/ui';
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
	<button type="button" class="chip" aria-haspopup="listbox" aria-expanded={open} title={m['creator.sound_mood']()} onclick={() => (open = !open)} data-testid="sound-mood">
		{#if face(shown)}<img src={face(shown)} alt="" />{/if}
		<span class="k">{m['creator.sound_mood']()}</span>
		<span>{value === 'auto' ? m['creator.sound_mood_auto']() : value}</span>
	</button>
	{#if open}
		<ul class="faces" role="listbox" aria-label={m['creator.sound_mood']()}>
			{#each ['auto', ...moods] as mood (mood)}
				<li>
					<button type="button" role="option" aria-selected={value === mood} class:on={value === mood} onclick={() => pick(mood)} data-testid="sound-mood-{mood}">
						{#if mood !== 'auto' && face(mood)}<img src={face(mood)} alt="" />{:else if mood === 'auto' && face(playing)}<img class="auto" src={face(playing)} alt="" />{:else}<span class="blank"></span>{/if}
						<span>{mood === 'auto' ? m['creator.sound_mood_auto']() : mood}</span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.wrap {
		position: relative;
	}
	.chip {
		all: unset;
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		height: 2rem;
		padding: 0 0.5rem 0 0.25rem;
		cursor: pointer;
		color: var(--paper);
		background: color-mix(in srgb, var(--ink) 70%, transparent);
		border: 1px solid color-mix(in srgb, var(--paper) 45%, transparent);
		font-size: 0.75rem;
	}
	.chip:focus-visible,
	.chip[aria-expanded='true'] {
		border-color: var(--paper);
	}
	.chip img {
		width: 1.5rem;
		height: 1.5rem;
		object-fit: cover;
		background: var(--paper);
	}
	.k {
		color: color-mix(in srgb, var(--paper) 60%, transparent);
	}
	.faces {
		position: absolute;
		z-index: 5;
		left: 0;
		top: calc(100% + 0.25rem);
		margin: 0;
		padding: 0.375rem;
		list-style: none;
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		width: min(19rem, 70vw);
		box-sizing: border-box;
		gap: 0.25rem;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		max-height: 16rem;
		overflow-y: auto;
	}
	.faces button {
		all: unset;
		box-sizing: border-box;
		width: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.125rem;
		padding: 0.25rem 0;
		cursor: pointer;
		font-size: 0.6875rem;
		text-align: center;
	}
	.faces button span:last-child {
		max-width: 100%;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.faces button:hover,
	.faces button:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: -2px;
	}
	.faces button.on {
		background: var(--ink);
		color: var(--paper);
	}
	.faces img,
	.faces .blank {
		width: 2.5rem;
		height: 2.5rem;
		object-fit: cover;
		background: var(--surface-panel);
	}
	.faces img.auto {
		opacity: 0.55;
	}
</style>
