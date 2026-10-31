<script lang="ts">
	// The deck's character, superimposed on the 3D (#198, owner 2026-10-31: "the character switcher could be
	// superimposed"): a chip on top with the face and the name; a tap opens the list (a bottom sheet).
	import { Sheet } from '@scutoons/ui';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	type Character = { id: string; name?: Record<string, string> };

	let {
		current,
		characters,
		face,
		mood,
		onpick
	}: {
		current: string;
		characters: Character[];
		/** the current mood's portrait (a CDN URL), if any */
		face: string;
		mood: string;
		onpick: (id: string) => void;
	} = $props();

	let open = $state(false);
	const nameOf = (c: Character) => c.name?.[getLocale()] || c.name?.en || c.id;
	const shown = $derived(characters.find((c) => c.id === current) ?? { id: current });

	function pick(id: string) {
		open = false;
		if (id !== current) onpick(id);
	}
</script>

<button class="chip" type="button" onclick={() => (open = true)} title={m['live_deck.choose_character']()} data-testid="deck-character">
	{#if face}<img src={face} alt="" crossorigin="anonymous" />{/if}
	<span class="name">{nameOf(shown)}</span>
	<span class="mood">{mood}</span>
</button>

<Sheet bind:open side="bottom" title={m['live_deck.choose_character']()} closeLabel={m['common.close']()}>
	<div class="list" role="listbox" aria-label={m['live_deck.character']()}>
		{#each characters as c (c.id)}
			<button type="button" role="option" aria-selected={c.id === current} onclick={() => pick(c.id)}>
				{nameOf(c)}
			</button>
		{/each}
	</div>
</Sheet>

<style>
	.chip {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		height: var(--control-height);
		max-width: calc(100vw - 2 * var(--control-height) - 4rem);
		padding: 0 var(--space-075) 0 0;
		border: var(--border-width-regular) solid var(--paper);
		background: var(--ink);
		color: var(--paper);
		font: inherit;
		cursor: pointer;
		overflow: hidden;
	}
	.chip img {
		height: 100%;
		aspect-ratio: 1;
		object-fit: cover;
		background: color-mix(in srgb, var(--paper) 10%, transparent);
	}
	.chip .name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.chip .mood {
		color: color-mix(in srgb, var(--paper) 60%, transparent);
		font-size: 0.875rem;
		white-space: nowrap;
	}
	.list {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
		gap: var(--space-05);
	}
	.list button {
		height: var(--control-height);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		text-align: left;
		padding: 0 var(--space-075);
		cursor: pointer;
	}
	/* the current one: inverted (DESIGN.md "Selection") */
	.list button[aria-selected='true'] {
		background: var(--ink);
		color: var(--paper);
	}
</style>
