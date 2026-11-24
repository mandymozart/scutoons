<script lang="ts">
	import type { LibraryEntry } from '@scutoons/api';
	import { library } from '$lib/editor/library.svelte.ts';
	import { typeRuns } from '$lib/editor/library-list.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LibraryTile from '$lib/components/library/library-tile.svelte';

	/**
	 * Entries of the Library's answer as tiles (or rows), the panel's and a stepped-into thing's: one list, or "By type"
	 * the runs of one type or kind under its word. Drawn a page at a time as the end scrolls into view. A picker hands in
	 * `onpress` (a press picks) and `chosen` (what its slot holds).
	 */
	let {
		entries,
		tile,
		list = false,
		grouped = false,
		label,
		onopen,
		onmenu,
		onpress,
		chosen
	}: {
		entries: readonly LibraryEntry[];
		tile: number;
		list?: boolean;
		grouped?: boolean;
		label: string;
		onopen: (entry: LibraryEntry) => void;
		onmenu?: (x: number, y: number) => void;
		onpress?: (entry: LibraryEntry) => void;
		chosen?: string | null;
	} = $props();

	const PAGE = 120;
	let shown = $state(PAGE);
	let end = $state<HTMLElement>();
	const visible = $derived(entries.slice(0, shown));
	const order = $derived(entries.map((e) => e.address));
	const runs = $derived(grouped ? typeRuns(visible) : [{ key: '', entries: visible }]);

	$effect(() => {
		void entries;
		shown = PAGE;
	});
	$effect(() => {
		const node = end;
		if (!node) return;
		const io = new IntersectionObserver((seen) => {
			if (seen.some((s) => s.isIntersecting)) shown += PAGE;
		});
		io.observe(node);
		return () => io.disconnect();
	});

	const runLabel = (key: string) => (key in TYPE_WORDS ? TYPE_WORDS[key]() : m[`library.kinds_${key as 'image'}`]());
	const TYPE_WORDS: Record<string, () => string> = {
		toon: m['library.types_toon'],
		character: m['library.types_character'],
		scene: m['library.types_scene'],
		show: m['library.types_show'],
		place: m['library.types_place'],
		item: m['library.types_item'],
		module: m['library.types_module'],
		instrument: m['library.types_instrument'],
		rig: m['library.types_rig'],
		generator: m['library.types_generator'],
		kanon: m['library.types_kanon'],
		person: m['library.types_person']
	};
</script>

{#each runs as run (run.key)}
	<section class="run" data-testid="library-run">
		{#if run.key}<h3 class="head">{runLabel(run.key)}</h3>{/if}
		<div class="tiles" class:list role="listbox" aria-multiselectable="true" aria-label={run.key ? runLabel(run.key) : label} style:--tile="{tile}rem">
			{#each run.entries as e (e.address)}
				<LibraryTile
					entry={e}
					row={list}
					selected={chosen !== undefined ? chosen === e.address : library.chosen.includes(e.address)}
					fresh={library.fresh === e.address}
					onpress={(k) => (onpress ? onpress(e) : library.click(e.address, k, order))}
					onopen={() => onopen(e)}
					{onmenu}
				/>
			{/each}
		</div>
	</section>
{/each}
{#if shown < entries.length}<div class="end" bind:this={end} aria-hidden="true"></div>{/if}

<style>
	.run {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.head {
		margin: 0;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		font-weight: 400;
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(var(--tile), 1fr));
		gap: 0.6rem 0.45rem;
	}
	.tiles.list {
		grid-template-columns: 1fr;
		gap: 0;
	}
	.end {
		height: 1px;
	}
</style>
