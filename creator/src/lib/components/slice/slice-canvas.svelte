<script lang="ts" module>
	export type ItemKind = 'dialog' | 'sample' | 'event' | 'item';
	export interface CanvasSelection {
		kind: ItemKind;
		index: number;
	}
	export type Tool = 'text' | 'sample' | 'item' | 'event';
</script>

<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { SliceLayers, type ResolveSliceItem } from '@scutoons/reader';
	import { itemCatalog, itemName, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DEFAULT_SLICE, getAt, round2, sliceLayers, type Path } from '@scutoons/content';
	import { SLICE_RATIO } from '$lib/editor/defaults.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { replacedImages } from '$lib/editor/replaced-images.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import { sampleResolver } from '$lib/editor/sample-readers.ts';
	import type { TextExpressions } from '@scutoons/reader';
	import { deltas, dialogScale, gridSteps, rotationAngle, scaleFactor, scaledRect, transformedDeltas, type Handle, type Rect } from '$lib/editor/canvas-geometry.ts';

	let {
		doc,
		slicePath,
		locale,
		grid = 0,
		tool = 'text',
		layers = null,
		selection = null,
		live = true,
		onselect,
		oncreate
	}: {
		doc: ContentDoc;
		slicePath: Path;
		/** the locale shown (text + its geometry) */
		locale: string;
		/** snap step in px of the slice's 1x size (0 = off; owner 2026-10-03: the slices' 15 px base grid) */
		grid?: number;
		tool?: Tool;
		/**
		 * the kinds the inspector's tab is on (owner 2026-10-03: full-slice events covered everything; the Layers tab = text
		 * boxes + placed items): only their boxes take the pointer, the others show faintly; null = every kind
		 */
		layers?: readonly ItemKind[] | null;
		selection?: CanvasSelection | null;
		/**
		 * the canvas is the live region (DESIGN.md "Selection": canvas objects can't be inverted): selected = the kind's
		 * frame; live = the frame + handles + its tag (the keys act on it)
		 */
		live?: boolean;
		onselect?: (selection: CanvasSelection | null) => void;
		/** a rect was drawn on empty space */
		oncreate?: (tool: Tool, rect: Required<Rect>) => void;
	} = $props();

	/** the slice as the reader shows it in `locale` (texts resolved – the editing below writes the document itself) */
	const slice = $derived<any>(doc.resolvedSlice(slicePath, locale));
	/** text boxes with `{…}` as the toolbar's sample reader sees them (agents/EXPRESSIONS.md step 4), or as written */
	const expressions = $derived.by<TextExpressions | undefined>(() => {
		const sample = exprContext.sample;
		if (sample === 'written') return undefined;
		return { resolver: (_text, lang) => sampleResolver(sample, lang ?? locale) };
	});
	const image = $derived(slice?.image ? replacedImages.src(asset(`${doc.toon}/${slice.image}`), `${doc.toon}/${slice.image}`) : '');
	/** the slice's 1x size (standard 390 × 625) – the canvas never shows it larger */
	const sliceWidth = $derived(slice?.width > 0 ? slice.width : DEFAULT_SLICE.width);
	const sliceRatio = $derived(slice?.width > 0 && slice?.height > 0 ? slice.height / slice.width : SLICE_RATIO);

	interface Item {
		kind: ItemKind;
		index: number;
		path: Path;
		rect: Rect;
		label: string;
		/** degrees (text boxes, placed items) */
		rotation: number;
		/** text boxes' uniform scale */
		scale: number;
		/** a text box a showTextBox event reveals: which one ("⚡ 2" on the slice, "script 3" in the group's script) */
		revealedBy?: string;
	}

	/** text box id → the showTextBox event that reveals it (`params.dialog`): the slice's events, its placed items' tap events, the group script */
	const revealers = $derived.by(() => {
		const out = new Map<string, string>();
		const add = (events: any, label: (i: number) => string) =>
			(Array.isArray(events) ? events : []).forEach((e: any, i: number) => {
				const ref = e?.type === 'showTextBox' ? e?.params?.dialog : undefined;
				if (typeof ref === 'string' && ref && !out.has(ref)) out.set(ref, label(i));
			});
		add(slice?.events, (i) => `⚡ ${i + 1}`);
		(slice?.items ?? []).forEach((it: any, n: number) => add(it?.events, (i) => `◆ ${n + 1}.${i + 1}`));
		const group = slicePath.length >= 2 ? doc.get(doc.primary, [...slicePath.slice(0, -2), 'events']) : null;
		add(group, (i) => `script ${i + 1}`);
		return out;
	});

	/** text boxes and placed items can be rotated and scaled on the canvas */
	const transformable = (item: Item) => item.kind === 'dialog' || item.kind === 'item';

	const num = (v: unknown, fallback: number) => (typeof v === 'number' && Number.isFinite(v) ? v : fallback);

	const items = $derived.by<Item[]>(() => {
		const out: Item[] = [];
		(slice?.dialog ?? []).forEach((d: any, i: number) => {
			const path = [...slicePath, 'dialog', i, 'rect'];
			const revealedBy = typeof d?.id === 'string' ? revealers.get(d.id) : undefined;
			if (d?.rect) out.push({ kind: 'dialog', index: i, path, rect: d.rect, label: d.id ?? `#${i + 1}`, rotation: num(d.rotation, 0), scale: num(d.scale, 1), revealedBy });
		});
		(slice?.items ?? []).forEach((it: any, i: number) => {
			const path = [...slicePath, 'items', i, 'rect'];
			const def = itemCatalog.catalog?.items.find((x) => x.id === it?.item);
			if (it?.rect) out.push({ kind: 'item', index: i, path, rect: it.rect, label: `◆ ${itemName(def, it.item ?? '?')}`, rotation: num(it.rotation, 0), scale: 1 });
		});
		(slice?.audio?.audio?.samples ?? []).forEach((s: any, i: number) => {
			const path = [...slicePath, 'audio', 'audio', 'samples', i, 'trigger', 'rect'];
			if (s?.trigger?.rect) out.push({ kind: 'sample', index: i, path, rect: s.trigger.rect, label: `♪ ${s.id} (${s.trigger.type})`, rotation: 0, scale: 1 });
		});
		(slice?.events ?? []).forEach((e: any, i: number) => {
			const path = [...slicePath, 'events', i, 'trigger', 'rect'];
			if (e?.trigger?.rect) out.push({ kind: 'event', index: i, path, rect: e.trigger.rect, label: `⚡ ${e.type} (${e.trigger.type})`, rotation: 0, scale: 1 });
		});
		return out;
	});

	/**
	 * drawing order: the big areas (events, samples) under the layer stack, larger first; text boxes and placed items in
	 * the slice's one layer order (`sliceLayers`, as the reader draws them); the selected one last (on top)
	 */
	const KIND_ORDER: Record<ItemKind, number> = { event: 0, sample: 1, item: 2, dialog: 2 };
	const layerRank = $derived(new Map(sliceLayers(slice).map((ref, rank) => [`${ref.kind}${ref.index}`, rank])));
	const ordered = $derived(
		[...items].sort((a, b) => {
			const sel = (i: Item) => (selection?.kind === i.kind && selection?.index === i.index ? 1 : 0);
			const area = (i: Item) => i.rect.width * (i.rect.height ?? 10);
			const rank = (i: Item) => layerRank.get(`${i.kind}${i.index}`) ?? 0;
			return sel(a) - sel(b) || KIND_ORDER[a.kind] - KIND_ORDER[b.kind] || (KIND_ORDER[a.kind] === 2 ? rank(a) - rank(b) : area(b) - area(a));
		})
	);
	/** a pointer on the canvas takes the keyboard from a field (Delete / Ctrl+X / arrows then act on the selection) */
	function releaseField() {
		const el = document.activeElement as HTMLElement | null;
		if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable)) el.blur();
	}

	let canvas = $state<HTMLDivElement | null>(null);
	let overlay = $state<HTMLDivElement | null>(null);
	let imageLoaded = $state(false);
	let imageError = $state(false);
	/** rendered heights (% of the slice) of text boxes without rect.height, by text box index */
	let measured = $state<Record<number, number>>({});
	/** rendered heights (% of the slice) of placed items without rect.height (the icon's ratio), by item index */
	let measuredItems = $state<Record<number, number>>({});

	function measure() {
		if (!canvas) return;
		const height = canvas.clientHeight;
		if (!height) return;
		const boxes: Record<number, number> = {};
		for (const el of canvas.querySelectorAll<HTMLElement>('.slice-layers > .dialog')) boxes[Number(el.dataset.index)] = round2((el.offsetHeight / height) * 100);
		measured = boxes;
		const next: Record<number, number> = {};
		for (const el of canvas.querySelectorAll<HTMLElement>('.slice-layers > .slice-item')) {
			if (el.offsetHeight) next[Number(el.dataset.index)] = round2((el.offsetHeight / height) * 100);
		}
		measuredItems = next;
	}

	$effect(() => {
		// re-measure when the text, geometry or locale change (a resize is caught by the ResizeObserver)
		void [JSON.stringify(slice?.dialog ?? []), JSON.stringify(slice?.items ?? []), itemCatalog.catalog, locale, imageLoaded];
		tick().then(() => requestAnimationFrame(measure));
	});

	/** placed items: the item's icon from the catalog (the same component as the reader) */
	const resolveItem: ResolveSliceItem = (it) => {
		const def = itemCatalog.catalog?.items.find((x) => x.id === it.item);
		return def?.icon ? { src: asset(def.icon), label: itemName(def, it.item) } : null;
	};

	onMount(() => {
		const observer = new ResizeObserver(() => requestAnimationFrame(measure));
		if (canvas) observer.observe(canvas);
		document.fonts?.addEventListener('loadingdone', measure);
		return () => {
			observer.disconnect();
			document.fonts?.removeEventListener('loadingdone', measure);
		};
	});

	$effect(() => {
		void image;
		imageLoaded = false;
		imageError = false;
	});

	$effect(() => {
		if (slice?.items?.length) loadCatalog();
	});

	function heightOf(item: Item) {
		if (item.rect.height !== undefined) return item.rect.height;
		if (item.kind === 'item') return measuredItems[item.index] ?? round2(item.rect.width / sliceRatio);
		return item.kind === 'dialog' ? (measured[item.index] ?? 5) : 5;
	}

	// ── Pointer interaction ────────────────────────────────────────────────────────────────────────────

	const HANDLES: Handle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];
	/** handles per kind: placed items keep their icon's ratio (move, rotate, scale), text boxes get all */
	const handlesOf = (item: Item): Handle[] => (item.kind === 'item' ? [] : HANDLES);

	interface Drag {
		mode: 'edit' | 'draw';
		handle: Handle;
		item?: Item;
		start: { x: number; y: number };
		/** start rect per locale that gets written */
		startRects: Record<string, Rect>;
		/** start rect of the shown locale (what the pointer snaps against) */
		shown: Required<Rect>;
		draw?: Required<Rect>;
		moved: boolean;
		/** the slice in px at the start (rotation and scale work in px: the slice isn't square) */
		px: { width: number; height: number };
	}
	let drag = $state<Drag | null>(null);

	function toPercent(event: PointerEvent) {
		const box = overlay!.getBoundingClientRect();
		return { x: ((event.clientX - box.left) / box.width) * 100, y: ((event.clientY - box.top) / box.height) * 100 };
	}

	/** the grid in % of the slice (lib/editor/canvas-geometry.ts) */
	const steps = $derived(gridSteps(grid, sliceWidth, sliceRatio));
	const stepX = $derived(steps.stepX);
	const stepY = $derived(steps.stepY);
	const snapX = (value: number) => steps.snapX(value);
	const snapY = (value: number) => steps.snapY(value);

	/** locales an edit writes to */
	function targets(): string[] {
		// geometry is the same in every language
		return doc.present;
	}

	function beginEdit(event: PointerEvent, item: Item, handle: Handle) {
		releaseField();
		event.stopPropagation();
		event.preventDefault();
		onselect?.({ kind: item.kind, index: item.index });
		const startRects: Record<string, Rect> = {};
		for (const l of targets()) {
			const rect = doc.get(l, item.path);
			if (rect) startRects[l] = { ...rect };
		}
		drag = {
			mode: 'edit',
			handle,
			item,
			start: toPercent(event),
			startRects,
			shown: { ...item.rect, height: heightOf(item) } as Required<Rect>,
			moved: false,
			px: overlaySize()
		};
		capture(event);
	}

	/** keep receiving moves outside the canvas (fails for synthetic events – harmless) */
	function capture(event: PointerEvent) {
		try {
			overlay?.setPointerCapture(event.pointerId);
		} catch {
			/* not a real pointer */
		}
	}

	function beginDraw(event: PointerEvent) {
		if (event.button !== 0) return;
		releaseField();
		onselect?.(null);
		const p = toPercent(event);
		const x = snapX(p.x);
		const y = snapY(p.y);
		drag = { mode: 'draw', handle: 'se', start: { x, y }, startRects: {}, shown: { x, y, width: 0, height: 0 }, draw: { x, y, width: 0, height: 0 }, moved: false, px: overlaySize() };
		capture(event);
	}

	function overlaySize() {
		const box = overlay?.getBoundingClientRect();
		return { width: box?.width || 1, height: box?.height || 1 };
	}

	/** the entry a rect belongs to (text box / placed item) in one locale */
	const entryOf = (l: string, item: Item) => doc.get(l, item.path.slice(0, -1));

	/** rotate handle: the angle from the centre to the pointer (0° = straight up); with snap in 15° steps, Shift switches */
	function rotateTo(event: PointerEvent, d: Drag) {
		const deg = rotationAngle(toPercent(event), d.shown, d.px, grid > 0 !== event.shiftKey);
		for (const l of Object.keys(d.startRects)) {
			const entry = entryOf(l, d.item!);
			if (!entry) continue;
			if (deg === 0) delete entry.rotation;
			else entry.rotation = deg;
		}
	}

	/**
	 * scale handle: the distance centre → pointer against the start. Text boxes: `scale` (the laid-out box grows, text
	 * keeps its wrapping); placed items: their width (and height), around the centre.
	 */
	function scaleTo(event: PointerEvent, d: Drag) {
		const f = scaleFactor(d.start, toPercent(event), d.shown, d.px);
		if (f === null) return;
		const item = d.item!;
		for (const [l, start] of Object.entries(d.startRects)) {
			if (item.kind === 'dialog') {
				const entry = entryOf(l, item);
				if (!entry) continue;
				const next = dialogScale(item.scale, f, grid > 0);
				if (next === 1) delete entry.scale;
				else entry.scale = next;
			} else {
				const rect = doc.get(l, item.path);
				if (!rect) continue;
				const next = scaledRect(start, d.shown.height, f, snapX);
				rect.x = next.x;
				rect.y = next.y;
				rect.width = next.width;
				if (next.height !== undefined) rect.height = next.height;
			}
		}
	}

	let checkpointed = false;

	function onPointerMove(event: PointerEvent) {
		if (!drag) return;
		const p = toPercent(event);
		const dx = p.x - drag.start.x;
		const dy = p.y - drag.start.y;
		if (!drag.moved && Math.abs(dx) < 0.3 && Math.abs(dy) < 0.3) return;
		drag.moved = true;
		if (drag.mode === 'draw') {
			const x2 = snapX(p.x);
			const y2 = snapY(p.y);
			drag.draw = {
				x: Math.min(drag.start.x, x2),
				y: Math.min(drag.start.y, y2),
				width: Math.abs(x2 - drag.start.x),
				height: Math.abs(y2 - drag.start.y)
			};
			return;
		}
		if (!checkpointed) {
			// one undo step per drag
			doc.checkpoint();
			checkpointed = true;
		}
		if (drag.handle === 'rotate') return rotateTo(event, drag);
		if (drag.handle === 'scale') return scaleTo(event, drag);
		const turned = drag.handle !== 'move' && (drag.item!.rotation !== 0 || drag.item!.scale !== 1);
		const { ddx, ddy, dw, dh } = turned
			? transformedDeltas(drag.handle, dx, dy, drag.shown, drag.item!, drag.px, grid > 0 ? steps : null)
			: deltas(drag.handle, dx, dy, drag.shown, steps);
		for (const [l, start] of Object.entries(drag.startRects)) {
			const rect = doc.get(l, drag.item!.path);
			if (!rect) continue;
			rect.x = round2(start.x + ddx);
			rect.y = round2(start.y + ddy);
			rect.width = round2(Math.max(1, start.width + dw));
			if (dh !== 0 || (drag.handle !== 'move' && /[ns]/.test(drag.handle))) {
				const base = start.height ?? drag.shown.height;
				rect.height = round2(Math.max(1, base + dh));
			}
		}
	}

	function onPointerUp(event: PointerEvent) {
		if (!drag) return;
		try {
			overlay?.releasePointerCapture(event.pointerId);
		} catch {
			/* not captured */
		}
		if (drag.mode === 'draw' && drag.draw && drag.draw.width >= 2 && drag.draw.height >= 1) {
			const r = drag.draw;
			oncreate?.(tool, { x: round2(r.x), y: round2(r.y), width: round2(r.width), height: round2(r.height) });
		} else if (drag.mode === 'draw' && (tool === 'item' || tool === 'event') && !drag.moved) {
			// a click is enough: the item tool places the item there; the event tool makes a whole-slice event (no area)
			oncreate?.(tool, { x: round2(drag.start.x), y: round2(drag.start.y), width: 0, height: 0 });
		}
		drag = null;
		checkpointed = false;
	}

	// ── Keyboard nudging (the page forwards keys when the canvas has the selection) ───────────────────

	/** moves the selection by px of the slice's 1x size (with snap: onto the grid) */
	export function nudge(dxPx: number, dyPx: number) {
		const dx = (dxPx / sliceWidth) * 100;
		const dy = (dyPx / (sliceWidth * sliceRatio)) * 100;
		const item = items.find((i) => i.kind === selection?.kind && i.index === selection?.index);
		if (!item) return false;
		doc.checkpoint(`nudge:${item.path.join('.')}`);
		for (const l of targets()) {
			const rect = doc.get(l, item.path);
			if (!rect) continue;
			rect.x = round2(dx ? snapX(rect.x + dx) : rect.x);
			rect.y = round2(dy ? snapY(rect.y + dy) : rect.y);
		}
		return true;
	}

	/** the grid's lines in % (every 4th step a stronger one) */
	const gridLines = $derived.by(() => {
		const lines = (step: number) => {
			const out: { v: number; major: boolean }[] = [];
			if (step > 0) for (let n = 1; n * step < 100; n++) out.push({ v: round2(n * step), major: n % 4 === 0 });
			return out;
		};
		return { x: lines(stepX), y: lines(stepY) };
	});
</script>

<!-- one resolution in the whole app (owner 2026-09-30): the slice at its 1x size, scaled down to fit the column (its width and, in the slice editor, its height) -->
<div class="canvas-wrap" style:max-width="min({sliceWidth}px, calc(100cqh * {1 / sliceRatio}))">
	<div
		class="slice creator-canvas"
		bind:this={canvas}
		style:aspect-ratio={imageLoaded ? null : `${sliceWidth} / ${sliceWidth * sliceRatio}`}
		data-testid="slice-canvas"
	>
		{#if image}
			<img
				src={image}
				alt=""
				draggable="false"
				onload={() => (imageLoaded = true)}
				onerror={() => (imageError = true)}
			/>
		{/if}
		{#if imageError}
			<p class="image-error">Image not found: {slice?.image}</p>
		{/if}
		<SliceLayers dialogs={slice?.dialog} items={slice?.items} lang={locale} {expressions} resolve={resolveItem} />

		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="overlay tool-{tool}"
			bind:this={overlay}
			onpointerdown={beginDraw}
			onpointermove={onPointerMove}
			onpointerup={onPointerUp}
			onpointercancel={onPointerUp}
		>
			<!-- the grid shows while snap is on (owner 2026-10-03), stronger while something is dragged -->
			{#if gridLines.x.length}
				<svg class="grid" class:dragging={drag?.moved} viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
					{#each gridLines.x as l (l.v)}<line x1={l.v} y1="0" x2={l.v} y2="100" class:major={l.major} />{/each}
					{#each gridLines.y as l (l.v)}<line x1="0" y1={l.v} x2="100" y2={l.v} class:major={l.major} />{/each}
				</svg>
			{/if}
			{#each ordered as item (item.kind + item.index)}
				{@const selected = selection?.kind === item.kind && selection?.index === item.index}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="box kind-{item.kind}"
					class:selected
					class:live={selected && live}
					class:inactive={layers !== null && !layers.includes(item.kind)}
					style:left="{item.rect.x}%"
					style:top="{item.rect.y}%"
					style:width="{item.rect.width}%"
					style:height="{heightOf(item)}%"
					class:auto-height={item.rect.height === undefined && item.kind !== 'item'}
					style:transform={item.rotation || item.scale !== 1 ? `rotate(${item.rotation}deg) scale(${item.scale})` : null}
					style:--box-scale={item.scale}
					data-testid="box-{item.kind}-{item.index}"
					onpointerdown={(e) => beginEdit(e, item, 'move')}
				>
					<span class="tag">{item.label}</span>
					{#if item.revealedBy}
						<span class="tag hidden-until" data-testid="hidden-until-{item.index}">{m['creator.reveal_hidden_until']({ event: item.revealedBy })}</span>
					{/if}
					{#if selected && live}
						{#each handlesOf(item) as h}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<span class="handle h-{h}" onpointerdown={(e) => beginEdit(e, item, h)}></span>
						{/each}
						{#if transformable(item)}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<span class="handle round h-rotate" title={m['creator.canvas_rotate']()} data-testid="handle-rotate" onpointerdown={(e) => beginEdit(e, item, 'rotate')}></span>
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<span class="handle round h-scale" title={m['creator.canvas_scale']()} data-testid="handle-scale" onpointerdown={(e) => beginEdit(e, item, 'scale')}></span>
						{/if}
					{/if}
				</div>
			{/each}
			{#if drag?.mode === 'draw' && drag.draw && drag.moved}
				<div
					class="box ghost kind-{tool === 'sample' ? 'sample' : tool === 'item' ? 'item' : tool === 'event' ? 'event' : 'dialog'}"
					style:left="{drag.draw.x}%"
					style:top="{drag.draw.y}%"
					style:width="{drag.draw.width}%"
					style:height="{drag.draw.height}%"
				></div>
			{/if}
		</div>
	</div>
</div>

<style>
	.canvas-wrap {
		position: relative;
		flex: none;
		width: 100%;
		margin: 0 auto;
	}
	.creator-canvas {
		position: relative;
		width: 100%;
		/* same as the reader's .slice: dialog font sizes are cqw */
		container-type: inline-size;
		background: var(--surface-panel);
		user-select: none;
	}
	.creator-canvas img {
		display: block;
		width: 100%;
		height: auto;
		pointer-events: none;
	}
	.image-error {
		position: absolute;
		inset: 40% 1rem auto;
		text-align: center;
		color: var(--error);
	}
	.overlay {
		position: absolute;
		inset: 0;
		z-index: 3;
		cursor: crosshair;
		touch-action: none;
	}
	.grid {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.grid {
		opacity: 0.5;
	}
	.grid.dragging {
		opacity: 1;
	}
	.grid line {
		stroke: rgba(0, 209, 255, 0.35);
		stroke-width: 1px;
		vector-effect: non-scaling-stroke;
	}
	.grid line.major {
		stroke: rgba(0, 209, 255, 0.7);
	}
	/* another layer than the inspector's tab: visible, but it lets the pointer through */
	.box.inactive {
		pointer-events: none;
		opacity: 0.3;
	}
	.box {
		position: absolute;
		box-sizing: border-box;
		border: 1px dashed rgba(0, 0, 0, 0.55);
		outline: 1px dashed rgba(255, 255, 255, 0.8);
		outline-offset: -2px;
		cursor: move;
	}
	.box.kind-dialog {
		border-color: var(--cyan);
		border-style: solid;
	}
	.box.kind-sample {
		border-color: var(--acid-purple);
		background: rgba(183, 10, 255, 0.08);
	}
	.box.kind-event {
		border-color: var(--state-error);
		background: rgba(255, 33, 10, 0.06);
	}
	.box.auto-height {
		border-bottom-style: dotted;
	}
	.box.selected {
		border-width: 2px;
		border-style: solid;
		background: rgba(0, 209, 255, 0.08);
		z-index: 2;
	}
	.box.ghost {
		border-style: dashed;
		pointer-events: none;
	}
	.tag {
		position: absolute;
		left: -1px;
		bottom: 100%;
		font-size: var(--type-caption-size);
		line-height: 1.2;
		padding: 0 0.3em;
		background: var(--ink);
		color: var(--paper);
		white-space: nowrap;
		pointer-events: none;
		opacity: 0;
	}
	/* a text box a showTextBox event reveals: always marked, under the box */
	.tag.hidden-until {
		bottom: auto;
		top: 100%;
		opacity: 1;
		background: var(--paper);
		color: var(--ink);
		border: 1px dashed var(--ink);
	}
	.box:hover .tag,
	.box.live .tag {
		opacity: 1;
	}
	.handle {
		position: absolute;
		width: 10px;
		height: 10px;
		background: var(--paper);
		border: 2px solid var(--ink);
		box-sizing: border-box;
		transform: translate(-50%, -50%);
	}
	/* handles keep their size on a scaled box */
	.box .handle {
		transform: translate(-50%, -50%) scale(calc(1 / var(--box-scale, 1)));
	}
	.handle.round {
		border-radius: 50%;
	}
	/* rotate: above the top edge; scale: off the bottom-right corner */
	.h-rotate { left: 50%; top: calc(-1.25rem / var(--box-scale, 1)); cursor: grab; }
	.h-scale { left: calc(100% + 0.6rem / var(--box-scale, 1)); top: calc(100% + 0.6rem / var(--box-scale, 1)); cursor: nwse-resize; }
	.box.kind-item {
		border-color: var(--ink);
		border-style: solid;
	}
	.h-nw { left: 0; top: 0; cursor: nwse-resize; }
	.h-n { left: 50%; top: 0; cursor: ns-resize; }
	.h-ne { left: 100%; top: 0; cursor: nesw-resize; }
	.h-e { left: 100%; top: 50%; cursor: ew-resize; }
	.h-se { left: 100%; top: 100%; cursor: nwse-resize; }
	.h-s { left: 50%; top: 100%; cursor: ns-resize; }
	.h-sw { left: 0; top: 100%; cursor: nesw-resize; }
	.h-w { left: 0; top: 50%; cursor: ew-resize; }
</style>
