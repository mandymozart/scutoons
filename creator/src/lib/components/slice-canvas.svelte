<script lang="ts" module>
	export type ItemKind = 'dialog' | 'sample' | 'event' | 'item';
	export interface CanvasSelection {
		kind: ItemKind;
		index: number;
	}
	export type Tool = 'text' | 'sample';
	export type Scope = 'all' | 'locale';
</script>

<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Dialogs, SliceItems, type ResolveSliceItem } from '@scutoons/reader';
	import { itemCatalog, itemName, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DEFAULT_SLICE, getAt, round2, type Path } from '@scutoons/content';
	import { SLICE_RATIO } from '$lib/editor/defaults.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	interface Rect {
		x: number;
		y: number;
		width: number;
		height?: number;
	}

	let {
		doc,
		slicePath,
		locale,
		grid = 0,
		tool = 'text',
		scope = 'all',
		selection = null,
		onselect,
		oncreate
	}: {
		doc: ContentDoc;
		slicePath: Path;
		/** the locale shown (text + its geometry) */
		locale: string;
		/** snap step in % (0 = off) */
		grid?: number;
		tool?: Tool;
		/** geometry edits: every locale (keeping per-locale differences) or only the shown locale */
		scope?: Scope;
		selection?: CanvasSelection | null;
		onselect?: (selection: CanvasSelection | null) => void;
		/** a rect was drawn on empty space */
		oncreate?: (tool: Tool, rect: Required<Rect>) => void;
	} = $props();

	const slice = $derived(doc.get(locale, slicePath) ?? doc.get(doc.primary, slicePath));
	const image = $derived(slice?.image ? asset(`${doc.toon}/${slice.image}`) : '');
	/** the slice's 1x size (standard 390 × 625) – the canvas never shows it larger */
	const sliceWidth = $derived(slice?.width > 0 ? slice.width : DEFAULT_SLICE.width);
	const sliceRatio = $derived(slice?.width > 0 && slice?.height > 0 ? slice.height / slice.width : SLICE_RATIO);

	interface Item {
		kind: ItemKind;
		index: number;
		path: Path;
		rect: Rect;
		label: string;
		overridden: string[];
		/** degrees (text boxes, placed items) */
		rotation: number;
		/** text boxes' uniform scale */
		scale: number;
		/** a text box an onomatopoeia event reveals: which one ("⚡ 2" on the slice, "script 3" in the group's script) */
		revealedBy?: string;
	}

	/** text box id → the onomatopoeia event that reveals it (`params.dialog`): the slice's events, then the group script */
	const revealers = $derived.by(() => {
		const out = new Map<string, string>();
		const add = (events: any, label: (i: number) => string) =>
			(Array.isArray(events) ? events : []).forEach((e: any, i: number) => {
				const ref = e?.type === 'showOnomatopeia' ? e?.params?.dialog : undefined;
				if (typeof ref === 'string' && ref && !out.has(ref)) out.set(ref, label(i));
			});
		add(slice?.events, (i) => `⚡ ${i + 1}`);
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
			if (d?.rect) out.push({ kind: 'dialog', index: i, path, rect: d.rect, label: d.id ?? `#${i + 1}`, overridden: doc.overrides(path), rotation: num(d.rotation, 0), scale: num(d.scale, 1), revealedBy });
		});
		(slice?.items ?? []).forEach((it: any, i: number) => {
			const path = [...slicePath, 'items', i, 'rect'];
			const def = itemCatalog.catalog?.items.find((x) => x.id === it?.item);
			if (it?.rect) out.push({ kind: 'item', index: i, path, rect: it.rect, label: `◆ ${itemName(def, it.item ?? '?')}`, overridden: doc.overrides(path), rotation: num(it.rotation, 0), scale: 1 });
		});
		(slice?.audio?.audio?.samples ?? []).forEach((s: any, i: number) => {
			const path = [...slicePath, 'audio', 'audio', 'samples', i, 'trigger', 'rect'];
			if (s?.trigger?.rect) out.push({ kind: 'sample', index: i, path, rect: s.trigger.rect, label: `♪ ${s.id} (${s.trigger.type})`, overridden: doc.overrides(path), rotation: 0, scale: 1 });
		});
		(slice?.events ?? []).forEach((e: any, i: number) => {
			const path = [...slicePath, 'events', i, 'trigger', 'rect'];
			if (e?.trigger?.rect) out.push({ kind: 'event', index: i, path, rect: e.trigger.rect, label: `⚡ ${e.type} (${e.trigger.type})`, overridden: doc.overrides(path), rotation: 0, scale: 1 });
		});
		return out;
	});

	let canvas = $state<HTMLDivElement | null>(null);
	let overlay = $state<HTMLDivElement | null>(null);
	let imageLoaded = $state(false);
	let imageError = $state(false);
	/** rendered heights (% of the slice) of text boxes without rect.height */
	let measured = $state<number[]>([]);
	/** rendered heights (% of the slice) of placed items without rect.height (the icon's ratio), by item index */
	let measuredItems = $state<Record<number, number>>({});

	function measure() {
		if (!canvas) return;
		const height = canvas.clientHeight;
		if (!height) return;
		const els = canvas.querySelectorAll<HTMLElement>('.dialogs > .dialog');
		measured = Array.from(els).map((el) => round2((el.offsetHeight / height) * 100));
		const next: Record<number, number> = {};
		for (const el of canvas.querySelectorAll<HTMLElement>('.slice-items > .slice-item')) {
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

	type Handle = 'move' | 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw' | 'rotate' | 'scale';
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

	const snap = (value: number) => (grid > 0 ? Math.round(value / grid) * grid : value);

	/** locales an edit writes to */
	function targets(): string[] {
		return scope === 'locale' ? [locale] : doc.present;
	}

	function beginEdit(event: PointerEvent, item: Item, handle: Handle) {
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
		onselect?.(null);
		const p = toPercent(event);
		const x = snap(p.x);
		const y = snap(p.y);
		drag = { mode: 'draw', handle: 'se', start: { x, y }, startRects: {}, shown: { x, y, width: 0, height: 0 }, draw: { x, y, width: 0, height: 0 }, moved: false, px: overlaySize() };
		capture(event);
	}

	/** new geometry of the shown rect for a pointer delta (snapped), as offsets to apply to every target */
	function deltas(handle: Handle, dx: number, dy: number, s: Required<Rect>) {
		let ddx = 0, ddy = 0, dw = 0, dh = 0;
		if (handle === 'move') {
			ddx = snap(s.x + dx) - s.x;
			ddy = snap(s.y + dy) - s.y;
		} else {
			if (handle.includes('e')) dw = Math.max(1 - s.width, snap(s.x + s.width + dx) - (s.x + s.width));
			if (handle.includes('w')) {
				ddx = Math.min(s.width - 1, snap(s.x + dx) - s.x);
				dw = -ddx;
			}
			if (handle.includes('s')) dh = Math.max(1 - s.height, snap(s.y + s.height + dy) - (s.y + s.height));
			if (handle.includes('n')) {
				ddy = Math.min(s.height - 1, snap(s.y + dy) - s.y);
				dh = -ddy;
			}
		}
		return { ddx, ddy, dw, dh };
	}

	function overlaySize() {
		const box = overlay?.getBoundingClientRect();
		return { width: box?.width || 1, height: box?.height || 1 };
	}

	const RAD = Math.PI / 180;

	/**
	 * Resize of a rotated / scaled box: the pointer delta in the box's own frame (un-rotated, un-scaled), the dragged
	 * edges follow it and the opposite edges stay where they are on screen (the box turns around its centre). No grid snap.
	 */
	function transformedDeltas(handle: Handle, dx: number, dy: number, s: Required<Rect>, item: Item, px: { width: number; height: number }) {
		const W = px.width / 100;
		const H = px.height / 100;
		const a = item.rotation * RAD;
		const k = item.scale || 1;
		// pointer delta (px) → local frame
		const sx = dx * W;
		const sy = dy * H;
		const lx = (Math.cos(a) * sx + Math.sin(a) * sy) / k;
		const ly = (-Math.sin(a) * sx + Math.cos(a) * sy) / k;
		const w = s.width * W;
		const h = s.height * H;
		let l = -w / 2, r = w / 2, t = -h / 2, b = h / 2;
		const min = W; // 1 % of the slice width
		if (handle.includes('e')) r = Math.max(l + min, r + lx);
		if (handle.includes('w')) l = Math.min(r - min, l + lx);
		if (handle.includes('s')) b = Math.max(t + H, b + ly);
		if (handle.includes('n')) t = Math.min(b - H, t + ly);
		const nw = r - l;
		const nh = b - t;
		// the new centre: the old one + the local offset, scaled and rotated back to the slice
		const ox = ((l + r) / 2) * k;
		const oy = ((t + b) / 2) * k;
		const cx = s.x * W + w / 2 + Math.cos(a) * ox - Math.sin(a) * oy;
		const cy = s.y * H + h / 2 + Math.sin(a) * ox + Math.cos(a) * oy;
		const nx = (cx - nw / 2) / W;
		const ny = (cy - nh / 2) / H;
		const horizontal = /[ew]/.test(handle);
		const vertical = /[ns]/.test(handle);
		return { ddx: nx - s.x, ddy: ny - s.y, dw: horizontal ? nw / W - s.width : 0, dh: vertical ? nh / H - s.height : 0 };
	}

	/** the entry a rect belongs to (text box / placed item) in one locale */
	const entryOf = (l: string, item: Item) => doc.get(l, item.path.slice(0, -1));

	/** rotate handle: the angle from the centre to the pointer (0° = straight up); Shift snaps to 15° */
	function rotateTo(event: PointerEvent, d: Drag) {
		const p = toPercent(event);
		const W = d.px.width / 100;
		const H = d.px.height / 100;
		const cx = (d.shown.x + d.shown.width / 2) * W;
		const cy = (d.shown.y + d.shown.height / 2) * H;
		let deg = Math.atan2(p.y * H - cy, p.x * W - cx) / RAD + 90;
		deg = event.shiftKey ? Math.round(deg / 15) * 15 : Math.round(deg);
		deg = ((((deg + 180) % 360) + 360) % 360) - 180;
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
		const p = toPercent(event);
		const W = d.px.width / 100;
		const H = d.px.height / 100;
		const cx = (d.shown.x + d.shown.width / 2) * W;
		const cy = (d.shown.y + d.shown.height / 2) * H;
		const d0 = Math.hypot(d.start.x * W - cx, d.start.y * H - cy);
		if (d0 < 2) return;
		const f = Math.hypot(p.x * W - cx, p.y * H - cy) / d0;
		const item = d.item!;
		for (const [l, start] of Object.entries(d.startRects)) {
			if (item.kind === 'dialog') {
				const entry = entryOf(l, item);
				if (!entry) continue;
				const next = round2(Math.min(10, Math.max(0.1, item.scale * f)));
				if (next === 1) delete entry.scale;
				else entry.scale = next;
			} else {
				const rect = doc.get(l, item.path);
				if (!rect) continue;
				const width = Math.max(1, start.width * f);
				const height = (start.height ?? d.shown.height) * (width / start.width);
				rect.x = round2(start.x + start.width / 2 - width / 2);
				rect.y = round2(start.y + (start.height ?? d.shown.height) / 2 - height / 2);
				rect.width = round2(width);
				if (start.height !== undefined) rect.height = round2(height);
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
			const x2 = snap(p.x);
			const y2 = snap(p.y);
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
			? transformedDeltas(drag.handle, dx, dy, drag.shown, drag.item!, drag.px)
			: deltas(drag.handle, dx, dy, drag.shown);
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
		}
		drag = null;
		checkpointed = false;
	}

	// ── Keyboard nudging (the page forwards keys when the canvas has the selection) ───────────────────

	export function nudge(dx: number, dy: number) {
		const item = items.find((i) => i.kind === selection?.kind && i.index === selection?.index);
		if (!item) return false;
		doc.checkpoint(`nudge:${item.path.join('.')}`);
		for (const l of targets()) {
			const rect = doc.get(l, item.path);
			if (!rect) continue;
			rect.x = round2(grid > 0 && dx ? snap(rect.x + dx) : rect.x + dx);
			rect.y = round2(grid > 0 && dy ? snap(rect.y + dy) : rect.y + dy);
		}
		return true;
	}

	const gridLines = $derived.by(() => {
		if (!grid) return [];
		const step = grid >= 2.5 ? grid : 5;
		const lines: number[] = [];
		for (let v = step; v < 100; v += step) lines.push(round2(v));
		return lines;
	});
</script>

<!-- one resolution in the whole app (owner 2026-09-30): the slice at its 1x size, scaled down to fit the column -->
<div class="canvas-wrap" style:max-width="{sliceWidth}px">
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
		<SliceItems items={slice?.items} resolve={resolveItem} />
		<Dialogs dialogs={slice?.dialog} lang={locale} />

		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="overlay tool-{tool}"
			bind:this={overlay}
			onpointerdown={beginDraw}
			onpointermove={onPointerMove}
			onpointerup={onPointerUp}
			onpointercancel={onPointerUp}
		>
			{#if gridLines.length}
				<svg class="grid" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
					{#each gridLines as v}
						<line x1={v} y1="0" x2={v} y2="100" class:major={v % 10 === 0} />
						<line x1="0" y1={v} x2="100" y2={v} class:major={v % 10 === 0} />
					{/each}
				</svg>
			{/if}
			{#each items as item (item.kind + item.index)}
				{@const selected = selection?.kind === item.kind && selection?.index === item.index}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="box kind-{item.kind}"
					class:selected
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
					<span class="tag">{item.label}{#if item.overridden.length}<em> · {item.overridden.join(',')}≠</em>{/if}</span>
					{#if item.revealedBy}
						<span class="tag hidden-until" data-testid="hidden-until-{item.index}">{m['creator.ono_hidden_until']({ event: item.revealedBy })}</span>
					{/if}
					{#if selected}
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
					class="box ghost kind-{tool === 'sample' ? 'sample' : 'dialog'}"
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
	.grid line {
		stroke: rgba(0, 209, 255, 0.35);
		stroke-width: 1px;
		vector-effect: non-scaling-stroke;
	}
	.grid line.major {
		stroke: rgba(0, 209, 255, 0.7);
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
	/* a text box an onomatopoeia event reveals: always marked, under the box */
	.tag.hidden-until {
		bottom: auto;
		top: 100%;
		opacity: 1;
		background: var(--paper);
		color: var(--ink);
		border: 1px dashed var(--ink);
	}
	.box:hover .tag,
	.box.selected .tag {
		opacity: 1;
	}
	.tag em {
		font-style: normal;
		color: var(--cyan);
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
