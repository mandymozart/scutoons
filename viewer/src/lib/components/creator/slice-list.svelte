<script lang="ts">
	import type { ContentDoc } from '$lib/creator/doc.svelte.ts';
	import { getAt, type Path } from '$lib/creator/paths.ts';
	import { newSlice } from '$lib/creator/defaults.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	let {
		doc,
		groupPath,
		chapterNumber,
		selected = -1,
		onselect
	}: {
		doc: ContentDoc;
		groupPath: Path;
		chapterNumber: number;
		selected?: number;
		onselect: (index: number) => void;
	} = $props();

	const slices = $derived<any[]>(doc.get(doc.primary, [...groupPath, 'slices']) ?? []);

	let dragFrom = $state<number | null>(null);
	let dragOver = $state<number | null>(null);

	function move(from: number, to: number) {
		if (from === to || from < 0) return;
		doc.structural((file) => {
			const list = getAt(file, [...groupPath, 'slices']);
			if (!Array.isArray(list) || from >= list.length) return;
			const [item] = list.splice(from, 1);
			list.splice(Math.min(to, list.length), 0, item);
		});
		onselect(to);
	}

	function add(after: number) {
		const at = after + 1;
		doc.structural((file) => {
			const path = [...groupPath, 'slices'];
			let list = getAt(file, path);
			if (!Array.isArray(list)) {
				list = [];
				// create the array in place
				const group = getAt(file, groupPath) ?? file;
				group.slices = list;
			}
			// new slices copy the previous image name pattern → next number
			const prev = list[at - 1]?.image as string | undefined;
			const next = prev?.match(/^(.*?)(\d+)(\.\w+)$/);
			const image = next ? `${next[1]}${Number(next[2]) + 1}${next[3]}` : newSlice(chapterNumber, at).image;
			list.splice(at, 0, { image });
		});
		onselect(at);
	}

	function remove(index: number) {
		const s = slices[index];
		if (!confirm(m['creator.remove_confirm']({ what: `slice ${index + 1} (${s?.image ?? ''})` }))) return;
		doc.structural((file) => {
			const list = getAt(file, [...groupPath, 'slices']);
			if (Array.isArray(list)) list.splice(index, 1);
		});
		onselect(Math.max(0, Math.min(index, slices.length - 1)));
	}

	const count = (s: any, key: string) => (Array.isArray(s?.[key]) ? s[key].length : 0);
</script>

<ol class="slices" data-testid="slice-list">
	{#each slices as slice, index (index)}
		<li
			class:selected={index === selected}
			class:over={dragOver === index && dragFrom !== index}
			draggable="true"
			ondragstart={(e) => {
				dragFrom = index;
				e.dataTransfer?.setData('text/plain', String(index));
			}}
			ondragover={(e) => {
				e.preventDefault();
				dragOver = index;
			}}
			ondragleave={() => (dragOver = dragOver === index ? null : dragOver)}
			ondrop={(e) => {
				e.preventDefault();
				if (dragFrom !== null) move(dragFrom, index);
				dragFrom = dragOver = null;
			}}
			ondragend={() => (dragFrom = dragOver = null)}
		>
			<button type="button" class="thumb" onclick={() => onselect(index)} data-testid="slice-{index}">
				<span class="no">{index + 1}</span>
				<img src={asset(`${doc.toon}/${slice?.image}`)} alt="" loading="lazy" draggable="false" />
				<span class="info">
					<span class="name">{slice?.image?.split('/').pop()}</span>
					<span class="marks">
						{#if count(slice, 'dialog')}<span title="text boxes">T{count(slice, 'dialog')}</span>{/if}
						{#if slice?.audio}<span title="audio">♪</span>{/if}
						{#if count(slice, 'events')}<span title="events">⚡{count(slice, 'events')}</span>{/if}
					</span>
				</span>
			</button>
			<span class="tools">
				<button type="button" class="ghost" title={m['creator.add_slice']()} onclick={() => add(index)}>+</button>
				<button type="button" class="ghost" title={m['creator.remove']()} onclick={() => remove(index)}>✕</button>
			</span>
		</li>
	{/each}
</ol>
{#if !slices.length}
	<button type="button" class="secondary" onclick={() => add(-1)}>+ {m['creator.add_slice']()}</button>
{/if}

<style>
	.slices {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}
	li {
		display: flex;
		align-items: stretch;
		border-bottom: 1px solid var(--border-subtle);
		border-left: 3px solid transparent;
	}
	li.selected {
		border-left-color: var(--ink);
		background: var(--surface-hover);
	}
	li.over {
		box-shadow: inset 0 3px 0 var(--cyan);
	}
	.thumb {
		flex: 1;
		display: flex;
		gap: 0.4rem;
		align-items: center;
		padding: 0.2rem;
		background: transparent;
		color: var(--ink);
		border: 0;
		text-align: left;
		min-width: 0;
		cursor: grab;
	}
	.no {
		width: 1.6rem;
		text-align: right;
		font-size: 10px;
		color: var(--muted);
	}
	img {
		width: 34px;
		height: 54px;
		object-fit: cover;
		background: var(--surface-panel);
	}
	.info {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.name {
		font-size: 10px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.marks {
		display: flex;
		gap: 0.3rem;
		font-size: 10px;
		color: var(--muted);
	}
	.tools {
		display: flex;
		flex-direction: column;
		justify-content: center;
		opacity: 0;
	}
	li:hover .tools,
	li.selected .tools {
		opacity: 1;
	}
	.tools button {
		font-size: 11px;
		padding: 0 0.35em;
	}
</style>
