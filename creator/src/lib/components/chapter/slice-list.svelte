<script lang="ts">
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { Icon, Menu, SortGrip, region, sortable } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DEFAULT_SLICE, getAt, type Path } from '@scutoons/content';
	import { fillSliceSize } from '$lib/editor/images.ts';
	import { newSlice } from '$lib/editor/defaults.ts';
	import { toonUrl } from '$lib/utils/cdn.ts';
	import { toonAsset } from '@scutoons/content';
	import { replacedImages } from '$lib/editor/replaced-images.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SliceImport from '$lib/components/chapter/slice-import.svelte';
	import { clickSelect, moveMany } from '$lib/editor/multi-select.ts';
	import { useClipHandler } from '$lib/editor/clipboard.svelte.ts';
	import { slicesClipHandler } from '$lib/editor/slices-clip.svelte.ts';
	import ReachChip, { type ReachMark } from '$lib/components/chapter/reach-chip.svelte';
	import Marks from '$lib/components/chapter/marks.svelte';
	import { sliceMarks } from '$lib/editor/slice-marks.ts';

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
	/**
	 * the film strip by shots (agents/research/TEXT-ACROSS-SLICES.md §10): a slice opens a new row where its storyboard
	 * shot (`slice.shot`) begins – the row carries the shot's id; a hand-made slice after a shot starts a row of its own
	 */
	const shotStart = (i: number) => i > 0 && (slices[i]?.shot ?? null) !== (slices[i - 1]?.shot ?? null);

	// the slice menu (•••): Replace picture … · Insert before … · Insert after … (files from the computer) · Add · Remove
	let filePick = $state<HTMLInputElement | null>(null);
	let fileFor: { index: number; zone: 'before' | 'on' | 'after' } | null = null;
	function pickFiles(index: number, zone: 'before' | 'on' | 'after') {
		fileFor = { index, zone };
		if (!filePick) return;
		filePick.multiple = zone !== 'on';
		filePick.value = '';
		filePick.click();
	}

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

	// the film strip follows the strip editor's open slice (scrolled in the sidebar only – never the shell around it)
	let list = $state<HTMLOListElement | null>(null);
	$effect(() => {
		const i = selected;
		const el = list?.children[i] as HTMLElement | undefined;
		if (!el) return;
		let scroller = el.parentElement;
		while (scroller && scroller !== document.body && !/(auto|scroll)/.test(getComputedStyle(scroller).overflowY)) scroller = scroller.parentElement;
		if (!scroller || scroller === document.body) return;
		const r = el.getBoundingClientRect();
		const box = scroller.getBoundingClientRect();
		if (r.top < box.top) scroller.scrollBy({ top: r.top - box.top, behavior: 'instant' });
		else if (r.bottom > box.bottom) scroller.scrollBy({ top: r.bottom - box.bottom, behavior: 'instant' });
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
		fillSliceSize(doc, [...groupPath, 'slices', at], (path) => toonUrl(doc.toon, path));
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

	// ── copy / cut / paste (BACKLOG 10.39 step 5): the picked slices (else the shown one) – slices-clip.svelte.ts ──
	useClipHandler(
		() => 'slices',
		slicesClipHandler({
			doc: () => doc,
			groupPath: () => groupPath,
			slices: () => slices,
			picked: () => (picked.length ? picked : selected >= 0 ? [selected] : []),
			pick: (next) => {
				picked = next;
				anchor = next[0] ?? null;
			},
			onselect: (index) => onselect(index)
		})
	);

</script>

<!-- the film strip (TEXT-ACROSS-SLICES.md §4.1, D1 = A): thumbnails in rows, a shot per row start -->
<ol class="slices" bind:this={list} use:sortable={{ onmove: moveSlices }} use:region={'slices'} data-testid="slice-list">
	{#each slices as slice, index (index)}
		<li
			data-sort-row
			class:sc-selected={picked.includes(index)}
			class:shown={index === selected}
			class:shot-start={shotStart(index)}
			data-shot={slice?.shot ?? null}
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
			{#if slice?.shot && (index === 0 || shotStart(index))}<span class="shot" data-testid="slice-shot-{index}">{slice.shot}</span>{/if}
			<button type="button" class="thumb" data-control-exempt="slice thumbnail card" onclick={(e) => pick(index, e)} title={slice?.image?.split('/').pop()} data-drawer-pick data-testid="slice-{index}">
				<span class="frame">
					<img src={replacedImages.src(toonUrl(doc.toon, slice?.image), toonAsset(doc.toon, slice?.image))} alt="" loading="lazy" draggable="false" />
					<span class="no">{index + 1}</span>
				</span>
				<span class="info">
					<span class="name">{slice?.image?.split('/').pop()}</span>
					<span class="marks">
						<ReachChip mark={markOf(index)} />
						<Marks marks={sliceMarks(slice)} testid="slice-marks-{index}" />
					</span>
				</span>
			</button>
			<span class="tools">
				<SortGrip label={m['creator.sort_move']({ name: `${index + 1} ${slice?.image?.split('/').pop() ?? ''}`.trim() })} title={m['creator.sort_hint']()} testid="slice-grip-{index}" />
				<Menu label={m['creator.slice_menu']({ number: index + 1 })} align="end">
					{#snippet trigger()}<Icon name="more-horizontal" label={m['creator.slice_menu']({ number: index + 1 })} />{/snippet}
					<button type="button" onclick={() => pickFiles(index, 'on')} data-testid="slice-replace-{index}">{m['creator.slice_replace']()}</button>
					<button type="button" onclick={() => pickFiles(index, 'before')} data-testid="slice-insert-before-{index}">{m['creator.slice_insert_before']()}</button>
					<button type="button" onclick={() => pickFiles(index, 'after')} data-testid="slice-insert-after-{index}">{m['creator.slice_insert_after']()}</button>
					<button type="button" onclick={() => add(index)} data-testid="slice-add-{index}">{m['creator.add_slice']()}</button>
					<button type="button" onclick={() => remove(index)} data-testid="slice-remove-{index}">{m['creator.slice_remove']()}</button>
				</Menu>
			</span>
		</li>
	{/each}
</ol>
{#if slices.length}
	<button type="button" class="add-last" title={m['creator.add_slice']()} aria-label={m['creator.add_slice']()} onclick={() => add(slices.length - 1)} data-testid="slice-add-last"><Icon name="plus" /></button>
{/if}
<input
	bind:this={filePick}
	class="file-pick"
	type="file"
	accept="image/jpeg,image/png,image/webp"
	tabindex="-1"
	aria-hidden="true"
	onchange={(e) => {
		const files = [...((e.currentTarget as HTMLInputElement).files ?? [])];
		if (files.length && fileFor) dropFiles(fileFor.index, files, fileFor.zone);
		fileFor = null;
	}}
/>
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
	/* the film strip: three thumbnails a row (the sidebar's width), a shot starts a row with its id above (§10) */
	.slices {
		list-style: none;
		margin: 0;
		padding: var(--space-05);
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: var(--space-05);
	}
	/* a card: picked = inverted, picked in the live list = the live look (tools.css .sc-selected) – around the thumbnail */
	li {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		padding: var(--space-025);
	}
	li.shot-start {
		grid-column-start: 1;
	}
	li[data-shot] {
		padding-top: 1.2em;
	}
	.shot {
		position: absolute;
		top: 0;
		left: 0;
		font-size: var(--type-caption-size);
		line-height: 1.2;
		white-space: nowrap;
		color: var(--text-muted);
	}
	/* image files from the computer: the drop line before / after a slice (new slices there) – the strip reads left to
	   right, so the line stands at the thumbnail's side */
	li.file-before {
		box-shadow: calc(-1 * var(--border-width-focus)) 0 0 var(--cyan);
	}
	li.file-after {
		box-shadow: var(--border-width-focus) 0 0 var(--cyan);
	}
	/* image files from the computer: the thumbnail framed in cyan (replace its image) */
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
		display: flex;
		flex-direction: column;
		align-items: stretch;
		width: 100%;
		gap: var(--space-025);
		padding: 0;
		background: transparent;
		color: inherit;
		border: 0;
		text-align: left;
		min-width: 0;
	}
	/* the slice's standard shape, also while its picture is missing */
	.frame {
		position: relative;
		display: block;
		width: 100%;
		aspect-ratio: 390 / 625;
		border: var(--border-width-hairline) solid var(--ink);
		background: var(--surface-panel);
	}
	img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.no {
		position: absolute;
		top: 0;
		left: 0;
		padding: 0 var(--space-025);
		background: var(--paper);
		color: var(--ink);
		border-right: var(--border-width-hairline) solid var(--ink);
		border-bottom: var(--border-width-hairline) solid var(--ink);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.info {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	/* the file name: the tooltip and screen readers (the number says where it is) */
	.name {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	.marks {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-025);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	/* the grip and the slice menu: over the thumbnail's top-right corner, on hover and on the shown slice */
	.tools {
		position: absolute;
		top: var(--space-025);
		right: var(--space-025);
		display: flex;
		align-items: center;
		background: var(--paper);
		border-left: var(--border-width-hairline) solid var(--ink);
		border-bottom: var(--border-width-hairline) solid var(--ink);
		opacity: 0;
	}
	li[data-shot] .tools {
		top: 1.2em;
	}
	li:hover .tools,
	li:focus-within .tools,
	li.shown .tools {
		opacity: 1;
	}
	/* small: the thumbnail stays visible under them */
	.tools :global(.trigger),
	.tools :global([data-sort-grip]) {
		width: auto;
		height: 1.5rem;
		min-width: 0;
		min-height: 0;
		padding: 0 var(--space-025);
		border: 0;
		background: transparent;
		color: var(--ink);
	}
	.add-last {
		display: flex;
		align-items: center;
		justify-content: center;
		width: calc(100% - 2 * var(--space-05));
		margin: 0 var(--space-05) var(--space-05);
		border: var(--border-width-hairline) dashed var(--border-muted);
		background: transparent;
		color: var(--text-muted);
	}
	.file-pick {
		display: none;
	}
</style>
