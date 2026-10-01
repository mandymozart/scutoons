<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DEFAULT_SLICE, getAt, type Path } from '@scutoons/content';
	import { fillSliceSize } from '$lib/editor/images.ts';
	import { newSlice } from '$lib/editor/defaults.ts';
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
			list.splice(at, 0, { image, width: DEFAULT_SLICE.width, height: DEFAULT_SLICE.height });
		});
		// the standard size until the real one is read from the image on the CDN
		fillSliceSize(doc, [...groupPath, 'slices', at], (path) => asset(`${doc.toon}/${path}`));
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
						{#if slice?.audio}<span title="audio"><Icon name="music" label="audio" /></span>{/if}
						{#if count(slice, 'events')}<span title="events"><Icon name="zap" label="events" />{count(slice, 'events')}</span>{/if}
					</span>
				</span>
			</button>
			<span class="tools">
				<button type="button" class="ghost" title={m['creator.add_slice']()} aria-label={m['creator.add_slice']()} onclick={() => add(index)}><Icon name="plus" /></button>
				<button type="button" class="ghost" title={m['creator.remove']()} aria-label={m['creator.remove']()} onclick={() => remove(index)}><Icon name="x" /></button>
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
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	/* selected = inverted (DESIGN.md "Brutalist tools") */
	li.selected {
		background: var(--ink);
		color: var(--paper);
	}
	li.selected .thumb,
	li.selected .no,
	li.selected .marks,
	li.selected .tools button {
		color: var(--paper);
	}
	li.over {
		box-shadow: inset 0 3px 0 var(--cyan);
	}
	.thumb {
		flex: 1;
		display: flex;
		justify-content: flex-start;
		gap: var(--space-05);
		align-items: center;
		padding: var(--space-025) var(--space-05);
		background: transparent;
		color: var(--ink);
		border: 0;
		text-align: left;
		min-width: 0;
		cursor: grab;
	}
	.no {
		flex: none;
		min-width: 1.5em;
		text-align: right;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-bold);
	}
	img {
		flex: none;
		width: 2.125rem;
		height: 3.375rem;
		object-fit: cover;
		background: var(--surface-panel);
	}
	.info {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.name {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.marks {
		display: flex;
		gap: var(--space-05);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-bold);
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
		min-width: var(--control-height);
		padding: 0;
	}
</style>
