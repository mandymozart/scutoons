<script lang="ts">
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { CloseIcon, Icon, SortGrip, region, sortable } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DEFAULT_SLICE, getAt, type Path } from '@scutoons/content';
	import { fillSliceSize } from '$lib/editor/images.ts';
	import { newSlice } from '$lib/editor/defaults.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { replacedImages } from '$lib/editor/replaced-images.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SliceImport from '$lib/components/chapter/slice-import.svelte';
	import { clickSelect, moveMany } from '$lib/editor/multi-select.ts';
	import ReachChip, { type ReachMark } from '$lib/components/chapter/reach-chip.svelte';

	let {
		doc,
		groupPath,
		chapterNumber,
		selected = -1,
		markOf = () => null,
		onselect
	}: {
		doc: ContentDoc;
		groupPath: Path;
		chapterNumber: number;
		selected?: number;
		/** a slice the reader can never reach gets a label (@scutoons/content reach.ts) */
		markOf?: (index: number) => ReachMark;
		onselect: (index: number) => void;
	} = $props();

	const slices = $derived<any[]>(doc.get(doc.primary, [...groupPath, 'slices']) ?? []);

	// multi-select (owner 2026-10-03): click = one, Shift+click = the range from the anchor, Ctrl / Cmd+click = add / remove;
	// dragging a picked slice by its grip (the shared Sortable) moves all picked ones (in their order). Every picked one
	// is `sc-selected` (DESIGN.md "Selection"): inverted, the live look while the list ('slices') is the live region
	let picked = $state<number[]>([]);
	let anchor = $state<number | null>(null);
	// image files dragged from the computer (owner 2026-10-06): onto the middle of a slice = replace its image, onto its
	// top / bottom edge = new slices before / after it (between two slices, before the first, after the last)
	let fileOver = $state<{ index: number; zone: 'before' | 'on' | 'after' } | null>(null);
	let importer = $state<SliceImport>();
	let emptyOver = $state(false);
	const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes('Files');
	function fileZone(e: DragEvent): 'before' | 'on' | 'after' {
		const box = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const y = (e.clientY - box.top) / box.height;
		return y < 0.25 ? 'before' : y > 0.75 ? 'after' : 'on';
	}
	function dropFiles(index: number, files: File[], zone: 'before' | 'on' | 'after') {
		if (zone === 'on') importer?.replace(index, files);
		else importer?.insert(zone === 'before' ? index : index + 1, files);
	}
	// the shown slice changed elsewhere (canvas prev / next, a problem link): it alone is picked
	$effect(() => {
		if (selected >= 0 && !picked.includes(selected)) {
			picked = [selected];
			anchor = selected;
		}
	});

	function pick(index: number, event: MouseEvent) {
		const next = clickSelect(picked, anchor, index, event);
		picked = next.selected;
		anchor = next.anchor;
		if (picked.includes(index)) onselect(index);
	}

	function moveSlices(from: number, to: number) {
		const moving = picked.length > 1 && picked.includes(from) ? [...picked] : [from];
		let map = new Map<number, number>();
		doc.edit((file) => {
			const list = getAt(file, [...groupPath, 'slices']);
			if (!Array.isArray(list)) return;
			const result = moveMany(list, moving, from, to);
			map = result.map;
			list.splice(0, list.length, ...result.list);
		});
		picked = moving.map((i) => map.get(i) ?? i).sort((a, b) => a - b);
		anchor = map.get(anchor ?? -1) ?? picked[0];
		onselect(map.get(selected) ?? picked[0]);
	}

	function add(after: number) {
		const at = after + 1;
		doc.edit((file) => {
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

	async function remove(index: number) {
		const s = slices[index];
		if (!(await confirmRemove(m['creator.remove_confirm']({ what: `slice ${index + 1} (${s?.image ?? ''})` })))) return;
		doc.edit((file) => {
			const list = getAt(file, [...groupPath, 'slices']);
			if (Array.isArray(list)) list.splice(index, 1);
		});
		onselect(Math.max(0, Math.min(index, slices.length - 1)));
	}

	const count = (s: any, key: string) => (Array.isArray(s?.[key]) ? s[key].length : 0);
</script>

<ol class="slices" use:sortable={{ onmove: moveSlices }} use:region={'slices'} data-testid="slice-list">
	{#each slices as slice, index (index)}
		<li
			data-sort-row
			class:sc-selected={picked.includes(index)}
			class:shown={index === selected}
			class:file-on={fileOver?.index === index && fileOver.zone === 'on'}
			class:file-before={fileOver?.index === index && fileOver.zone === 'before'}
			class:file-after={fileOver?.index === index && fileOver.zone === 'after'}
			ondragover={(e) => {
				if (!hasFiles(e)) return;
				e.preventDefault();
				fileOver = { index, zone: fileZone(e) };
				if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
			}}
			ondragleave={() => (fileOver = fileOver?.index === index ? null : fileOver)}
			ondrop={(e) => {
				const files = [...(e.dataTransfer?.files ?? [])];
				if (!files.length) return;
				e.preventDefault();
				dropFiles(index, files, fileZone(e));
				fileOver = null;
			}}
		>
			<SortGrip label={m['creator.sort_move']({ name: `${index + 1} ${slice?.image?.split('/').pop() ?? ''}`.trim() })} title={m['creator.sort_hint']()} testid="slice-grip-{index}" />
			<button type="button" class="thumb" data-control-exempt="slice thumbnail card" onclick={(e) => pick(index, e)} data-testid="slice-{index}">
				<span class="no">{index + 1}</span>
				<img src={replacedImages.src(asset(`${doc.toon}/${slice?.image}`), `${doc.toon}/${slice?.image}`)} alt="" loading="lazy" draggable="false" />
				<span class="info">
					<span class="name">{slice?.image?.split('/').pop()}</span>
					<span class="marks">
						<ReachChip mark={markOf(index)} />
						{#if count(slice, 'dialog')}<span title={m['creator.text_boxes']()}>T{count(slice, 'dialog')}</span>{/if}
						{#if slice?.audio}<span title={m['creator.audio']()}><Icon name="music" label={m['creator.audio']()} /></span>{/if}
						{#if count(slice, 'events')}<span title={m['creator.events']()}><Icon name="zap" label={m['creator.events']()} />{count(slice, 'events')}</span>{/if}
					</span>
				</span>
			</button>
			<span class="tools">
				<button type="button" class="ghost" title={m['creator.add_slice']()} aria-label={m['creator.add_slice']()} onclick={() => add(index)}><Icon name="plus" /></button>
				<button type="button" class="ghost" title={m['creator.remove']()} aria-label={m['creator.remove']()} onclick={() => remove(index)}><CloseIcon /></button>
			</span>
		</li>
	{/each}
</ol>
{#if !slices.length}
	<!-- an empty group: a dashed slot – add one slice, or drop images from the computer (new slices, in name order) -->
	<div
		class="empty"
		class:file-on={emptyOver}
		role="region"
		aria-label={m['creator.slices_drop_empty']()}
		ondragover={(e) => {
			if (!hasFiles(e)) return;
			e.preventDefault();
			emptyOver = true;
			if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
		}}
		ondragleave={() => (emptyOver = false)}
		ondrop={(e) => {
			e.preventDefault();
			emptyOver = false;
			const files = [...(e.dataTransfer?.files ?? [])];
			if (files.length) importer?.insert(0, files);
		}}
		data-testid="slice-drop-empty"
	>
		<button type="button" class="secondary" onclick={() => add(-1)}><Icon name="plus" /> {m['creator.add_slice']()}</button>
		<span class="sc-caption">{m['creator.slices_drop_empty']()}</span>
	</div>
{/if}
<SliceImport bind:this={importer} {doc} {groupPath} {chapterNumber} {selected} {onselect} />

<style>
	.slices {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}
	/* light dividers between a group's slices: the selected ones stand out (owner 2026-10-06, BACKLOG M.12; their look:
	   tools.css .sc-selected) */
	li {
		display: flex;
		align-items: stretch;
		border-bottom: var(--border-width-hairline) solid var(--border-muted);
	}
	/* image files from the computer: the drop line above / below a slice (new slices there) */
	li.file-before {
		box-shadow: inset 0 3px 0 var(--cyan);
	}
	li.file-after {
		box-shadow: inset 0 -3px 0 var(--cyan);
	}
	/* image files from the computer: the middle of a slice framed in cyan (replace its image); an edge shows the drop
	   line above (.file-before) or below (.file-after) – new slices there */
	li.file-on,
	.empty.file-on {
		outline: var(--border-width-focus) solid var(--cyan);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.empty {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-05);
		padding: var(--space-1);
		border: var(--border-width-hairline) dashed var(--ink);
	}
	.thumb {
		flex: 1;
		display: flex;
		justify-content: flex-start;
		gap: var(--space-05);
		align-items: center;
		padding: var(--space-025) var(--space-05);
		background: transparent;
		color: inherit;
		border: 0;
		text-align: left;
		min-width: 0;
	}
	.no {
		flex: none;
		min-width: 1.5em;
		text-align: right;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
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
	}
	.tools {
		display: flex;
		flex-direction: column;
		justify-content: center;
		opacity: 0;
	}
	/* the shown slice keeps its + / × at hand (E.8: it stands out by its tools, not by another look) */
	li:hover .tools,
	li.shown .tools {
		opacity: 1;
	}
	.tools button {
		min-width: var(--control-height);
		padding: 0;
		color: inherit;
	}
</style>
