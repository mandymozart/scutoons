<script lang="ts">
	// Draft (research/CAVE.md D4–D6): the panel next to / below the room – the opened furniture's contents or the
	// workbench's recipes, then the reader's inventory (select an item, then tap a rack or the room).
	import Glyph from './glyph.svelte';
	import { type Placed, type Thing, thing } from './cave-things.ts';

	let {
		visitor,
		note,
		open,
		carried = $bindable(),
		selected = $bindable(),
		ontakeout,
		onnote
	}: {
		visitor: boolean;
		note: string;
		open: Placed | null;
		carried: Thing[];
		selected: number | null;
		ontakeout: (p: Placed, t: Thing) => void;
		onnote: (note: string) => void;
	} = $props();

	function repair() {
		const broken = carried.find((c) => c.kind === 'broken_book');
		if (!broken) return;
		carried = carried.map((c) =>
			c.uid === broken.uid ? thing('songbook_diy', 'DIY song book', 'book') : c
		);
		onnote('Repaired – the song book is back in your inventory.');
	}
	const furnitureSelected = $derived(carried.find((c) => c.uid === selected)?.kind === 'furniture');
</script>

<aside class="panel" data-testid="cave-panel">
	{#if note}<p class="note" role="status">{note}</p>{/if}
	{#if open}
		<h2>
			{open.f.name}{#if open.f.holds}<span class="muted"
					>{` · `}{open.contents.length} / {open.f.holds.max}</span
				>{/if}
		</h2>
		{#if open.f.holds}
			<div class="grid">
				{#each open.contents as c (c.uid)}
					<button
						class="thing"
						disabled={visitor}
						title={visitor ? c.name : `Take out: ${c.name}`}
						onclick={() => ontakeout(open, c)}
					>
						<Glyph kind={c.item} /><span>{c.name}</span>
					</button>
				{/each}
			</div>
			{#if !visitor}<p class="muted">
					Select something in your inventory, then tap the {open.f.name}.
				</p>{/if}
		{:else if open.f.workshop}
			<p>Workshop recipes</p>
			<button
				class="primary"
				disabled={!carried.some((c) => c.kind === 'broken_book')}
				onclick={repair}
				data-testid="cave-repair"
			>
				Repair the broken song book
			</button>
		{/if}
	{/if}

	<h2>{visitor ? 'Your inventory (you are visiting)' : 'Your inventory'}</h2>
	<div class="grid">
		{#each carried as c (c.uid)}
			<button
				class="thing"
				class:selected={selected === c.uid}
				disabled={visitor}
				onclick={() => (selected = selected === c.uid ? null : c.uid)}
			>
				<Glyph kind={c.item} /><span>{c.name}</span>
			</button>
		{/each}
	</div>
	{#if furnitureSelected}<p class="muted">
			Tap a place in the room – the furniture stays in the room from then on.
		</p>{/if}
</aside>

<style>
	.panel {
		display: grid;
		gap: 0.5rem;
		align-content: start;
		overflow-y: auto;
		padding: 0 1rem 1rem;
	}
	@media (min-width: 64rem) {
		.panel {
			padding-top: 1rem;
			border-left: var(--border-width-regular) solid var(--ink);
		}
	}
	h2 {
		margin: 0.5rem 0 0;
		font-size: 1rem;
	}
	p {
		margin: 0;
	}
	.muted {
		color: var(--muted);
	}
	.note {
		padding: 0.5rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(5.5rem, 1fr));
		gap: 0.5rem;
	}
	.thing {
		display: grid;
		justify-items: center;
		gap: 0.25rem;
		padding: 0.25rem;
		border: var(--border-width-regular) solid transparent;
		background: none;
		color: var(--ink);
		font: inherit;
		font-size: 0.75rem;
		cursor: pointer;
	}
	.thing.selected {
		border-color: var(--ink);
	}
	.thing:disabled {
		cursor: default;
	}
	.primary {
		justify-self: start;
		min-height: var(--control-height);
		padding: 0 0.75rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--ink);
		color: var(--paper);
		font: inherit;
		cursor: pointer;
	}
	.primary:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
