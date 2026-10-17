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
	import { DEFAULT_SLICE, anchorAt, anchorFactors, anchoredRect, dialogFit, estimatedWidth, type DialogFit, getAt, round2, sliceLayers, suggestedAnchor, type Path } from '@scutoons/content';
	import { drawnHeights, drawnKey, drawnWidths } from '$lib/editor/drawn-heights.svelte.ts';
	import { SLICE_RATIO } from '$lib/editor/defaults.ts';
	import { asset, toonUrl } from '$lib/utils/cdn.ts';
	import { toonAsset } from '@scutoons/content';
	import { replacedImages } from '$lib/editor/replaced-images.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import { sampleResolver } from '$lib/editor/sample-readers.ts';
	import type { TextExpressions } from '@scutoons/reader';
	import CanvasTextEdit from '$lib/components/slice/canvas-text-edit.svelte';
	import { centreSnap, deltas, dialogScale, gridSteps, movedRect, rotationAngle, scaleFactor, scaledRect, transformedDeltas, type Factors, type Handle, type Rect } from '$lib/editor/canvas-geometry.ts';

	let {
		doc,
		slicePath,
		locale,
		grid = 0,
		tool = 'text',
		layers = null,
		selection = null,
		live = true,
		zoom = null,
		onshown,
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
		 * boxes + placed items + the click / hover areas of its stack): only their boxes take the pointer, the others show
		 * faintly; null = every kind
		 */
		layers?: readonly ItemKind[] | null;
		selection?: CanvasSelection | null;
		/**
		 * the canvas is the live region (DESIGN.md "Selection": canvas objects can't be inverted): selected = the kind's
		 * frame; live = the frame + handles + its tag (the keys act on it)
		 */
		live?: boolean;
		/**
		 * a factor of the slice's 1x width (UI-CONCEPTS E.4: 50–300 %, the area scrolls); null = fit – the area's height, at
		 * most 2x (on a phone its width). No CSS transform: the canvas is laid out at its size, so handles, snapping and the
		 * pointer stay in slice coordinates (%)
		 */
		zoom?: number | null;
		/** the factor the canvas is shown at (its width / the slice's 1x width), on every size change */
		onshown?: (factor: number) => void;
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
	const image = $derived(slice?.image ? replacedImages.src(toonUrl(doc.toon, slice.image), toonAsset(doc.toon, slice.image)) : '');
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
		/** where `rect.x` / `rect.y` sit in the box (text boxes, placed items: their `anchor`; areas: top-left) */
		f: Factors;
		/** a text box a showTextBox event reveals: which one ("⚡ 2" on the slice, "script 3" in the group's script) */
		revealedBy?: string;
		/** a text box's sizing (dialog-fit.ts); in auto width `rect.width` here is the drawn width, not the stored max */
		fit?: DialogFit;
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

	const TOP_LEFT: Factors = { ax: 0, ay: 0 };
	const num = (v: unknown, fallback: number) => (typeof v === 'number' && Number.isFinite(v) ? v : fallback);

	const items = $derived.by<Item[]>(() => {
		const out: Item[] = [];
		(slice?.dialog ?? []).forEach((d: any, i: number) => {
			const path = [...slicePath, 'dialog', i, 'rect'];
			const revealedBy = typeof d?.id === 'string' ? revealers.get(d.id) : undefined;
			if (!d?.rect) return;
			const fit = dialogFit(d);
			// the canvas works on the drawn box: auto width at its measured width, auto height without a stored height
			const rect: Rect =
				fit === 'width'
					? { x: d.rect.x, y: d.rect.y, width: measuredWidths[i] ?? estimatedWidth(d) }
					: fit === 'height'
						? { x: d.rect.x, y: d.rect.y, width: d.rect.width }
						: d.rect;
			out.push({ kind: 'dialog', index: i, path, rect, label: d.id ?? `#${i + 1}`, rotation: num(d.rotation, 0), scale: num(d.scale, 1), f: anchorFactors(d.anchor), revealedBy, fit });
		});
		(slice?.items ?? []).forEach((it: any, i: number) => {
			const path = [...slicePath, 'items', i, 'rect'];
			const def = itemCatalog.catalog?.items.find((x) => x.id === it?.item);
			if (it?.rect) out.push({ kind: 'item', index: i, path, rect: it.rect, label: `◆ ${itemName(def, it.item ?? '?')}`, rotation: num(it.rotation, 0), scale: 1, f: anchorFactors(it.anchor) });
		});
		(slice?.audio?.audio?.samples ?? []).forEach((s: any, i: number) => {
			const path = [...slicePath, 'audio', 'audio', 'samples', i, 'trigger', 'rect'];
			if (s?.trigger?.rect) out.push({ kind: 'sample', index: i, path, rect: s.trigger.rect, label: `♪ ${s.id} (${s.trigger.type})`, rotation: 0, scale: 1, f: TOP_LEFT });
		});
		(slice?.events ?? []).forEach((e: any, i: number) => {
			const path = [...slicePath, 'events', i, 'trigger', 'rect'];
			if (e?.trigger?.rect) out.push({ kind: 'event', index: i, path, rect: e.trigger.rect, label: `⚡ ${e.type} (${e.trigger.type})`, rotation: 0, scale: 1, f: TOP_LEFT });
		});
		return out;
	});

	/**
	 * drawing (= pointer) order: the slice's one layer stack (`sliceLayers` – text boxes, placed items and the click / hover
	 * areas of events and samples, BACKLOG 10.43 + 10.54) as the reader draws and hit-tests it, every event of an area at
	 * the area's place; under it the other areas (in view), larger first; the selected one last (on top)
	 */
	const layerRank = $derived(
		new Map(sliceLayers(slice).flatMap((ref, rank) => (ref.events ?? [ref.index]).map((index) => [`${ref.kind}${index}`, rank] as const)))
	);
	const inStack = (i: Item) => layerRank.has(`${i.kind}${i.index}`);
	const ordered = $derived(
		[...items].sort((a, b) => {
			const sel = (i: Item) => (selection?.kind === i.kind && selection?.index === i.index ? 1 : 0);
			const area = (i: Item) => i.rect.width * (i.rect.height ?? 10);
			const rank = (i: Item) => layerRank.get(`${i.kind}${i.index}`) ?? -1;
			return sel(a) - sel(b) || rank(a) - rank(b) || area(b) - area(a);
		})
	);
	/**
	 * a box takes the pointer on its tab; on the Layers tab (the one with the text boxes) only the areas of the stack, not
	 * a whole-slice / in-view event's
	 */
	const active = (i: Item) => layers === null || (layers.includes(i.kind) && (!layers.includes('dialog') || inStack(i)));
	/** a pointer on the canvas takes the keyboard from a field (Delete / Ctrl+X / arrows then act on the selection) */
	function releaseField() {
		const el = document.activeElement as HTMLElement | null;
		if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable)) el.blur();
	}

	let canvas = $state<HTMLDivElement | null>(null);
	let wrap = $state<HTMLDivElement | null>(null);
	$effect(() => {
		const el = wrap;
		const w = sliceWidth;
		if (!el || !onshown) return;
		const report = () => el.clientWidth > 0 && onshown(Math.round((el.clientWidth / w) * 100) / 100);
		const ro = new ResizeObserver(report);
		ro.observe(el);
		report();
		return () => ro.disconnect();
	});
	let overlay = $state<HTMLDivElement | null>(null);
	let imageLoaded = $state(false);
	let imageError = $state(false);
	/** rendered heights (% of the slice) of text boxes without rect.height, by text box index */
	let measured = $state<Record<number, number>>({});
	/** rendered widths (% of the slice) of auto-width text boxes (fit 'width'), by text box index */
	let measuredWidths = $state<Record<number, number>>({});
	/** rendered heights (% of the slice) of placed items without rect.height (the icon's ratio), by item index */
	let measuredItems = $state<Record<number, number>>({});

	function measure() {
		if (!canvas) return;
		const height = canvas.clientHeight;
		if (!height) return;
		const boxes: Record<number, number> = {};
		const widths: Record<number, number> = {};
		for (const el of canvas.querySelectorAll<HTMLElement>('.slice-layers > .dialog')) {
			boxes[Number(el.dataset.index)] = round2((el.offsetHeight / height) * 100);
			if (el.classList.contains('fit-width') && canvas.clientWidth) widths[Number(el.dataset.index)] = round2((el.offsetWidth / canvas.clientWidth) * 100);
		}
		measured = boxes;
		measuredWidths = widths;
		for (const [i, w] of Object.entries(widths)) drawnWidths[drawnKey([...slicePath, 'dialog', Number(i)])] = w;
		const next: Record<number, number> = {};
		for (const el of canvas.querySelectorAll<HTMLElement>('.slice-layers > .slice-item')) {
			if (el.offsetHeight) next[Number(el.dataset.index)] = round2((el.offsetHeight / height) * 100);
		}
		measuredItems = next;
		// for the anchor picker (keeps a box in place when its anchor changes)
		for (const [i, h] of Object.entries(boxes)) drawnHeights[drawnKey([...slicePath, 'dialog', Number(i)])] = h;
		for (const [i, h] of Object.entries(next)) drawnHeights[drawnKey([...slicePath, 'items', Number(i)])] = h;
	}

	$effect(() => {
		// re-measure when the text, geometry or locale change (a resize is caught by the ResizeObserver)
		void [JSON.stringify(slice?.dialog ?? []), JSON.stringify(slice?.items ?? []), itemCatalog.catalog, locale, imageLoaded];
		tick().then(() => requestAnimationFrame(measure));
	});

	/** placed items: the item's icon from the catalog (the same component as the reader) */
	const resolveItem: ResolveSliceItem = (it) => {
		const def = itemCatalog.catalog?.items.find((x) => x.id === it.item);
		// a placed variant shows its own picture (a colour of the sunglasses)
		const icon = (it.variant ? def?.variants?.find((v) => v.id === it.variant)?.icon : null) || def?.icon;
		return icon ? { src: asset(icon), label: itemName(def, it.item) } : null;
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

	/** the drawn box: its top-left corner and size (x / y place the anchor point – anchor.ts), what the handles work on */
	function boxOf(item: Item): Required<Rect> {
		const b = anchoredRect(item.rect, anchorAt(item.f.ax, item.f.ay), heightOf(item));
		return { x: b.left, y: b.top, width: b.width, height: b.height };
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
		/** start rect per locale that gets written (an auto-width box: at its drawn width) */
		startRects: Record<string, Rect>;
		/** the stored width per locale at the start (auto width: the max width, or none) */
		storedWidths: Record<string, number | undefined>;
		/** start rect of the shown locale (what the pointer snaps against) */
		shown: Required<Rect>;
		draw?: Required<Rect>;
		moved: boolean;
		/** the slice in px at the start (rotation and scale work in px: the slice isn't square) */
		px: { width: number; height: number };
	}
	let drag = $state<Drag | null>(null);
	/** the text box whose words are edited in place (a double click on it; owner 2026-10-17) */
	let editingText = $state<number | null>(null);
	$effect(() => {
		// another selection (or none) closes the in-place editor
		if (editingText !== null && !(selection?.kind === 'dialog' && selection.index === editingText)) editingText = null;
	});
	const BOXED = ['caption', 'narration', 'lyrics'];
	/** the stage's centre lines a moved box snapped to (owner 2026-10-17); Ctrl / Cmd while dragging = no centre snap */
	let guides = $state({ x: false, y: false });

	/**
	 * a hint only (owner 2026-10-17): a text box / placed item dragged to within 3 % of a slice edge or corner suggests
	 * the anchor there – nothing changes by itself, the inspector's picker sets it
	 */
	const anchorHint = $derived.by(() => {
		if (drag?.mode !== 'edit' || drag.handle !== 'move' || !drag.moved || !drag.item || !transformable(drag.item)) return null;
		const now = items.find((i) => i.kind === drag!.item!.kind && i.index === drag!.item!.index);
		if (!now) return null;
		const b = boxOf(now);
		return suggestedAnchor({ left: b.x, top: b.y, width: b.width, height: b.height }, anchorAt(now.f.ax, now.f.ay));
	});

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
		const storedWidths: Record<string, number | undefined> = {};
		for (const l of targets()) {
			const rect = doc.get(l, item.path);
			if (!rect) continue;
			storedWidths[l] = rect.width;
			// auto width: the drawn width stands in for the max width (the handles work on the drawn box)
			startRects[l] = item.fit === 'width' ? { x: rect.x, y: rect.y, width: item.rect.width } : { ...rect };
		}
		drag = {
			mode: 'edit',
			handle,
			item,
			start: toPercent(event),
			startRects,
			storedWidths,
			shown: boxOf(item),
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
		drag = { mode: 'draw', handle: 'se', start: { x, y }, startRects: {}, storedWidths: {}, shown: { x, y, width: 0, height: 0 }, draw: { x, y, width: 0, height: 0 }, moved: false, px: overlaySize() };
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
				const next = scaledRect(start, d.shown.height, f, snapX, item.f);
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
		let { ddx, ddy, dw, dh } = turned
			? transformedDeltas(drag.handle, dx, dy, drag.shown, drag.item!, drag.px, grid > 0 ? steps : null)
			: deltas(drag.handle, dx, dy, drag.shown, steps);
		guides = { x: false, y: false };
		if (drag.handle === 'move' && !(event.ctrlKey || event.metaKey)) {
			// within 0.5rem of the centre line (rem, not px – DESIGN.md), measured in % of each axis
			const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
			// against the pointer's own delta, so it also wins over the grid (a box off the grid could never be centred)
			const sx = centreSnap(drag.shown.x, drag.shown.width, dx, ((0.5 * rem) / drag.px.width) * 100);
			const sy = centreSnap(drag.shown.y, drag.shown.height, dy, ((0.5 * rem) / drag.px.height) * 100);
			if (sx.hit) ddx = sx.delta;
			if (sy.hit) ddy = sy.delta;
			guides = { x: sx.hit, y: sy.hit };
		}
		// the drawn box moves / resizes; the stored point follows its anchor (dragging the opposite edge keeps it)
		const writeHeight = dh !== 0 || (drag.handle !== 'move' && /[ns]/.test(drag.handle));
		for (const [l, start] of Object.entries(drag.startRects)) {
			const rect = doc.get(l, drag.item!.path);
			if (!rect) continue;
			const next = movedRect(start, { ddx, ddy, dw, dh }, drag.shown.height, writeHeight, drag.item!.f);
			rect.x = next.x;
			rect.y = next.y;
			rect.width = next.width;
			if (next.height !== undefined) rect.height = next.height;
			if (drag.item!.kind === 'dialog') resized(l, drag.item!, rect, drag.handle === 'move', writeHeight, drag.storedWidths[l]);
		}
	}

	/**
	 * A text box's sizing after a move / resize (owner 2026-10-17, like Figma; DESIGN.md Part 2 "Box sizing"): a move keeps
	 * the mode (auto width keeps its stored max width or none); a side handle on an auto-width box makes it auto height at
	 * that width; a height change (n / s / a corner) makes any box a fixed size. Auto height and fixed are stored as before
	 * – no `fit`, the height decides – only auto width writes `fit: 'width'`.
	 */
	function resized(l: string, item: Item, rect: any, moved: boolean, heightChanged: boolean, storedWidth: number | undefined) {
		const entry = entryOf(l, item);
		if (!entry) return;
		if (item.fit === 'width' && moved) {
			if (storedWidth === undefined) delete rect.width;
			else rect.width = storedWidth;
			return;
		}
		if (moved) return;
		if (heightChanged || item.fit === 'width') delete entry.fit;
		if (!heightChanged && item.fit !== 'fixed') delete rect.height;
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
		guides = { x: false, y: false };
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

<!-- one resolution in the whole app (owner 2026-09-30): the slice's 1x size (its own px – content, not a UI size) is the
     unit; fit = the area's height (a phone: its width), at most 2x (E.4); zoomed = 1x × the factor, the area scrolls -->
<div
	class="canvas-wrap"
	class:fit={zoom === null}
	bind:this={wrap}
	style:--slice-w="{sliceWidth}px"
	style:--fit-h="calc(100cqh * {1 / sliceRatio})"
	style:--zoom={zoom}
	data-testid="canvas-wrap"
>
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
			ondblclick={() => {
				// the dblclick lands on the overlay (it holds the pointer capture): the box the double click selected
				if (selection?.kind === 'dialog') editingText = selection.index;
			}}
		>
			<!-- the grid shows while snap is on (owner 2026-10-03), stronger while something is dragged -->
			{#if gridLines.x.length}
				<svg class="grid" class:dragging={drag?.moved} viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
					{#each gridLines.x as l (l.v)}<line x1={l.v} y1="0" x2={l.v} y2="100" class:major={l.major} />{/each}
					{#each gridLines.y as l (l.v)}<line x1="0" y1={l.v} x2="100" y2={l.v} class:major={l.major} />{/each}
				</svg>
			{/if}
			<!-- the stage's centre lines while a moved box sits on them (owner 2026-10-17) -->
			{#if drag?.moved && (guides.x || guides.y)}
				<svg class="guides" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true" data-testid="centre-guides">
					{#if guides.x}<line x1="50" y1="0" x2="50" y2="100" data-axis="x" />{/if}
					{#if guides.y}<line x1="0" y1="50" x2="100" y2="50" data-axis="y" />{/if}
				</svg>
			{/if}
			{#each ordered as item (item.kind + item.index)}
				{@const selected = selection?.kind === item.kind && selection?.index === item.index}
				{@const box = boxOf(item)}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="box kind-{item.kind}"
					class:selected
					class:live={selected && live}
					class:inactive={!active(item)}
					style:left="{box.x}%"
					style:top="{box.y}%"
					style:width="{box.width}%"
					style:height="{box.height}%"
					class:auto-height={item.rect.height === undefined && item.kind !== 'item'}
					style:transform={item.rotation || item.scale !== 1 ? `rotate(${item.rotation}deg) scale(${item.scale})` : null}
					style:--box-scale={item.scale}
					data-testid="box-{item.kind}-{item.index}"
					onpointerdown={(e) => beginEdit(e, item, 'move')}
				>
					{#if item.kind === 'dialog' && editingText === item.index}
						{@const d = slice?.dialog?.[item.index]}
						<CanvasTextEdit {doc} path={item.path.slice(0, -1)} {locale} dialog={d} boxed={BOXED.includes(d?.variant)} onclose={() => (editingText = null)} />
					{/if}
					<span class="tag">{item.label}</span>
					{#if item.revealedBy}
						<span class="tag hidden-until" data-testid="hidden-until-{item.index}">{m['creator.reveal_hidden_until']({ event: item.revealedBy })}</span>
					{/if}
					{#if anchorHint && drag?.item?.kind === item.kind && drag.item.index === item.index}
						<span class="tag anchor-hint" data-testid="anchor-hint">{m['creator.anchor_suggest']({ anchor: m[`creator.anchor_${anchorHint}`]() })}</span>
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
		width: calc(var(--slice-w) * var(--zoom, 1));
		margin: 0 auto;
	}
	.canvas-wrap.fit {
		width: 100%;
		max-width: min(calc(2 * var(--slice-w)), var(--fit-h));
	}
	/* a phone: fit = the full width (the area scrolls down – the owner's phone draft) */
	@media (max-width: 767px) {
		.canvas-wrap.fit {
			max-width: calc(2 * var(--slice-w));
		}
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
		/* cqw for the in-place text editor (the reader's text sizes are in cqw of the slice) */
		container-type: inline-size;
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
	/* the centre guides: above the boxes, 2px in the live selection's dark cyan – set apart from the pale grid */
	.guides {
		position: absolute;
		inset: 0;
		z-index: 2;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}
	.guides line {
		stroke: var(--select-live-text);
		stroke-width: 2px;
		vector-effect: non-scaling-stroke;
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
	/* the anchor suggestion while a box is dragged near an edge: under the box, quiet */
	.tag.anchor-hint {
		bottom: auto;
		top: 100%;
		opacity: 1;
		background: var(--paper);
		color: var(--ink);
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
	/* touch screens (BACKLOG 10.50 d): a bigger handle, a 2.75rem target around it; rotate / scale further out so their
	   targets don't cover the corner ones */
	@media (pointer: coarse) {
		.handle {
			width: 1rem;
			height: 1rem;
		}
		.handle::before {
			content: '';
			position: absolute;
			left: 50%;
			top: 50%;
			width: 2.75rem;
			height: 2.75rem;
			transform: translate(-50%, -50%);
		}
		.h-rotate { top: calc(-2.5rem / var(--box-scale, 1)); }
		.h-scale { left: calc(100% + 1.5rem / var(--box-scale, 1)); top: calc(100% + 1.5rem / var(--box-scale, 1)); }
	}
</style>
