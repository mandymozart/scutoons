<script lang="ts" module>
	export type ItemKind = 'dialog' | 'sample' | 'event';
	export interface CanvasSelection {
		kind: ItemKind;
		index: number;
	}
	export type Tool = 'text' | 'sample';
	export type Scope = 'all' | 'locale';
</script>

<script lang="ts">
	import { onMount, tick } from 'svelte';
	import Dialogs from '$lib/components/toon/dialog/dialogs.svelte';
	import type { ContentDoc } from '$lib/creator/doc.svelte.ts';
	import { getAt, round2, type Path } from '@scutoons/content';
	import { SLICE_RATIO } from '$lib/creator/defaults.ts';
	import { asset } from '$lib/utils/cdn.ts';

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
		zoom = 1,
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
		zoom?: 1 | 3;
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

	interface Item {
		kind: ItemKind;
		index: number;
		path: Path;
		rect: Rect;
		label: string;
		overridden: string[];
	}

	const items = $derived.by<Item[]>(() => {
		const out: Item[] = [];
		(slice?.dialog ?? []).forEach((d: any, i: number) => {
			const path = [...slicePath, 'dialog', i, 'rect'];
			if (d?.rect) out.push({ kind: 'dialog', index: i, path, rect: d.rect, label: d.id ?? `#${i + 1}`, overridden: doc.overrides(path) });
		});
		(slice?.audio?.audio?.samples ?? []).forEach((s: any, i: number) => {
			const path = [...slicePath, 'audio', 'audio', 'samples', i, 'trigger', 'rect'];
			if (s?.trigger?.rect) out.push({ kind: 'sample', index: i, path, rect: s.trigger.rect, label: `♪ ${s.id} (${s.trigger.type})`, overridden: doc.overrides(path) });
		});
		(slice?.events ?? []).forEach((e: any, i: number) => {
			const path = [...slicePath, 'events', i, 'trigger', 'rect'];
			if (e?.trigger?.rect) out.push({ kind: 'event', index: i, path, rect: e.trigger.rect, label: `⚡ ${e.type} (${e.trigger.type})`, overridden: doc.overrides(path) });
		});
		return out;
	});

	let canvas = $state<HTMLDivElement | null>(null);
	let overlay = $state<HTMLDivElement | null>(null);
	let imageLoaded = $state(false);
	let imageError = $state(false);
	/** rendered heights (% of the slice) of text boxes without rect.height */
	let measured = $state<number[]>([]);

	function measure() {
		if (!canvas) return;
		const height = canvas.clientHeight;
		if (!height) return;
		const els = canvas.querySelectorAll<HTMLElement>('.dialogs > .dialog');
		measured = Array.from(els).map((el) => round2((el.offsetHeight / height) * 100));
	}

	$effect(() => {
		// re-measure when the text, geometry, zoom or locale change
		void [JSON.stringify(slice?.dialog ?? []), zoom, locale, imageLoaded];
		tick().then(() => requestAnimationFrame(measure));
	});

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

	function heightOf(item: Item) {
		if (item.rect.height !== undefined) return item.rect.height;
		return item.kind === 'dialog' ? (measured[item.index] ?? 5) : 5;
	}

	// ── Pointer interaction ────────────────────────────────────────────────────────────────────────────

	type Handle = 'move' | 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';
	const HANDLES: Handle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

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
			moved: false
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
		drag = { mode: 'draw', handle: 'se', start: { x, y }, startRects: {}, shown: { x, y, width: 0, height: 0 }, draw: { x, y, width: 0, height: 0 }, moved: false };
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
		const { ddx, ddy, dw, dh } = deltas(drag.handle, dx, dy, drag.shown);
		if (!checkpointed) {
			// one undo step per drag
			doc.checkpoint();
			checkpointed = true;
		}
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

<div class="canvas-wrap" style:width="{390 * zoom}px">
	<div
		class="slice creator-canvas"
		bind:this={canvas}
		style:min-height={imageLoaded ? null : `${390 * zoom * SLICE_RATIO}px`}
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
					class:auto-height={item.rect.height === undefined}
					data-testid="box-{item.kind}-{item.index}"
					onpointerdown={(e) => beginEdit(e, item, 'move')}
				>
					<span class="tag">{item.label}{#if item.overridden.length}<em> · {item.overridden.join(',')}≠</em>{/if}</span>
					{#if selected}
						{#each HANDLES as h}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<span class="handle h-{h}" onpointerdown={(e) => beginEdit(e, item, h)}></span>
						{/each}
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
		border-color: #b70aff;
		background: rgba(183, 10, 255, 0.08);
	}
	.box.kind-event {
		border-color: #ff210a;
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
		font-size: 10px;
		line-height: 1.2;
		padding: 0 0.3em;
		background: var(--ink);
		color: var(--paper);
		white-space: nowrap;
		pointer-events: none;
		opacity: 0;
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
	.h-nw { left: 0; top: 0; cursor: nwse-resize; }
	.h-n { left: 50%; top: 0; cursor: ns-resize; }
	.h-ne { left: 100%; top: 0; cursor: nesw-resize; }
	.h-e { left: 100%; top: 50%; cursor: ew-resize; }
	.h-se { left: 100%; top: 100%; cursor: nwse-resize; }
	.h-s { left: 50%; top: 100%; cursor: ns-resize; }
	.h-sw { left: 0; top: 100%; cursor: nesw-resize; }
	.h-w { left: 0; top: 50%; cursor: ew-resize; }
</style>
