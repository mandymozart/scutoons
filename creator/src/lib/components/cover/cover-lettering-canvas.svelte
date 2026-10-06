<script lang="ts">
	import { tick } from 'svelte';
	import { CoverArt } from '@scutoons/reader';
	import { COVER_LIMITS, type CoverFormat, type CoverLine, type CoverTexts } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { sourceLabel } from '$lib/editor/cover-words.ts';

	/**
	 * The cover art with its lettering lines drawn over it (the reader's own CoverArt, the editing boxes on top – same
	 * geometry: % of the cover, cqw): drag a line to move it (snaps to the centre, Alt: free), handles for width / size /
	 * angle (Shift: 5° steps), arrow keys nudge (Shift: 5 %), Delete removes (own layout), Escape lets go.
	 */
	let {
		image,
		format,
		lines,
		mode,
		texts,
		lang,
		selected = $bindable(null),
		lineLabel,
		onedit,
		onown,
		onremove
	}: {
		image: string;
		format: CoverFormat;
		lines: CoverLine[];
		mode: 'off' | 'default' | 'custom';
		/** the preview language's titles */
		texts: CoverTexts;
		lang: string;
		selected?: number | null;
		lineLabel: (l: CoverLine) => string;
		onedit: (index: number, change: Partial<CoverLine>, key: string) => void;
		/** the default layout is touched: it becomes the format's own */
		onown: () => void;
		onremove: (index: number) => void;
	} = $props();

	const round = (v: number) => Math.round(v * 10) / 10;
	const clamp = (v: number, key: keyof typeof COVER_LIMITS) => Math.min(COVER_LIMITS[key].max, Math.max(COVER_LIMITS[key].min, v));

	let stage = $state<HTMLDivElement | null>(null);
	/** the lines' rendered heights in % of the cover's width (cqw), measured from the reader's own DOM */
	let heights = $state<number[]>([]);
	let guide = $state(false);

	async function measure() {
		await tick();
		if (!stage) return;
		const width = stage.clientWidth || 1;
		heights = lines.map((_, i) => {
			const text = stage!.querySelector<HTMLElement>(`[data-line="${i}"] .text`);
			return text ? (text.offsetHeight / width) * 100 : 0;
		});
	}
	$effect(() => {
		void JSON.stringify(lines);
		void texts.toon;
		void texts.season;
		void lang;
		measure();
		document.fonts?.ready.then(measure).catch(() => {});
	});

	type Drag = { kind: 'move' | 'width' | 'size' | 'rotate'; index: number; startX: number; startY: number; start: CoverLine; cx: number; cy: number; startDist: number };
	let drag: Drag | null = null;

	function startDrag(event: PointerEvent, index: number, kind: Drag['kind']) {
		if (event.button !== 0 || !stage) return;
		event.preventDefault();
		event.stopPropagation();
		// the default layout becomes the format's own as soon as it is touched
		if (mode === 'default') onown();
		selected = index;
		const rect = stage.getBoundingClientRect();
		const start = $state.snapshot(lines[index]) as CoverLine;
		const cx = rect.left + (start.x / 100) * rect.width;
		const cy = rect.top + (start.y / 100) * rect.height;
		drag = { kind, index, startX: event.clientX, startY: event.clientY, start, cx, cy, startDist: Math.hypot(event.clientX - cx, event.clientY - cy) || 1 };
		(event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
		(event.currentTarget as HTMLElement).closest<HTMLElement>('.box')?.focus({ preventScroll: true });
	}

	function onDrag(event: PointerEvent) {
		if (!drag || !stage) return;
		const rect = stage.getBoundingClientRect();
		const { kind, index, start } = drag;
		const key = `cover-drag-${kind}-${index}`;
		if (kind === 'move') {
			let x = clamp(start.x + ((event.clientX - drag.startX) / rect.width) * 100, 'x');
			const y = clamp(start.y + ((event.clientY - drag.startY) / rect.height) * 100, 'y');
			// snap to the centre line (Alt: free)
			guide = !event.altKey && Math.abs(x - 50) < 1.5;
			if (guide) x = 50;
			onedit(index, { x: round(x), y: round(y) }, key);
		} else if (kind === 'width') {
			const half = Math.abs(event.clientX - drag.cx);
			onedit(index, { maxWidth: round(clamp(((half * 2) / rect.width) * 100, 'maxWidth')) }, key);
		} else if (kind === 'size') {
			const dist = Math.hypot(event.clientX - drag.cx, event.clientY - drag.cy);
			onedit(index, { size: round(clamp(start.size * (dist / drag.startDist), 'size')) }, key);
		} else {
			// the handle sits above the line: straight up = 0°; Shift snaps to 5°
			let angle = (Math.atan2(event.clientY - drag.cy, event.clientX - drag.cx) * 180) / Math.PI + 90;
			if (angle > 180) angle -= 360;
			angle = event.shiftKey ? Math.round(angle / 5) * 5 : Math.round(angle);
			onedit(index, { rotate: clamp(angle, 'rotate') }, key);
		}
	}

	function endDrag() {
		drag = null;
		guide = false;
	}

	function onKey(event: KeyboardEvent, index: number) {
		const step = event.shiftKey ? 5 : 0.5;
		const moves: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
		if (moves[event.key]) {
			event.preventDefault();
			if (mode === 'default') onown();
			const [dx, dy] = moves[event.key];
			const current = lines[index];
			onedit(index, { x: round(clamp(current.x + dx, 'x')), y: round(clamp(current.y + dy, 'y')) }, `cover-nudge-${index}`);
		} else if ((event.key === 'Delete' || event.key === 'Backspace') && mode === 'custom') {
			event.preventDefault();
			onremove(index);
		} else if (event.key === 'Escape') selected = null;
	}
</script>

<div class="canvas">
	{#if image}
		<div class="stage {format}" bind:this={stage} data-testid="cover-stage" role="presentation" onpointermove={onDrag} onpointerup={endDrag} onpointercancel={endDrag} onpointerdown={() => (selected = null)}>
			<CoverArt src={image} {format} lettering={mode === 'off' ? false : { [format]: lines }} title={texts.toon} subtitle={texts.season} {lang} />
			<div class="boxes">
				{#if guide}<div class="guide" aria-hidden="true"></div>{/if}
				{#each lines as l, i (i)}
					{@const height = Math.max(heights[i] || l.size, 2)}
					<div
						class="box"
						class:selected={selected === i}
						style="left: {l.x}%; top: {l.y}%; width: {l.maxWidth}%; height: {height}cqw; transform: translate(-50%, -50%) rotate({l.rotate ?? 0}deg) skewX({l.skew ?? 0}deg)"
						role="button"
						tabindex="0"
						aria-label="{sourceLabel(l.text)}: {lineLabel(l)}"
						title="{sourceLabel(l.text)}: {lineLabel(l)}"
						onpointerdown={(e) => startDrag(e, i, 'move')}
						onkeydown={(e) => onKey(e, i)}
						onfocus={() => (selected = i)}
						data-testid="cover-box-{i}"
					>
						{#if selected === i}
							<span class="handle width" role="presentation" title={m['creator.cover_handle_width']()} onpointerdown={(e) => startDrag(e, i, 'width')} data-testid="cover-handle-width"></span>
							<span class="handle size" role="presentation" title={m['creator.cover_handle_size']()} onpointerdown={(e) => startDrag(e, i, 'size')} data-testid="cover-handle-size"></span>
							<span class="handle rotate" role="presentation" title={m['creator.cover_handle_rotate']()} onpointerdown={(e) => startDrag(e, i, 'rotate')} data-testid="cover-handle-rotate"></span>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	{:else}
		<p class="sc-empty">{m['creator.cover_no_art']()}</p>
	{/if}
</div>

<style>
	.canvas {
		min-width: 0;
	}
	.stage {
		position: relative;
		border: var(--border-width-regular) solid var(--ink);
		touch-action: none;
		user-select: none;
	}
	.boxes {
		position: absolute;
		inset: 0;
		container-type: inline-size;
	}
	.box {
		position: absolute;
		box-sizing: border-box;
		border: var(--border-width-regular) dashed rgb(255 255 255 / 0.7);
		outline: var(--border-width-hairline) dashed rgb(0 0 0 / 0.6);
		cursor: move;
	}
	.box:hover {
		border-style: solid;
	}
	.box.selected,
	.box:focus-visible {
		border: var(--border-width-regular) solid var(--primary-cyan);
		outline: var(--border-width-hairline) solid var(--ink);
	}
	.guide {
		position: absolute;
		left: 50%;
		top: 0;
		bottom: 0;
		border-left: var(--border-width-hairline) solid var(--primary-cyan);
		pointer-events: none;
	}
	.handle {
		position: absolute;
		width: 0.75rem;
		height: 0.75rem;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		box-sizing: border-box;
	}
	.handle.width {
		right: -0.4rem;
		top: 50%;
		transform: translateY(-50%);
		cursor: ew-resize;
	}
	.handle.size {
		right: -0.4rem;
		bottom: -0.4rem;
		cursor: nwse-resize;
		background: var(--ink);
	}
	.handle.rotate {
		left: 50%;
		top: -1.5rem;
		transform: translateX(-50%);
		border-radius: var(--radius-full);
		cursor: grab;
	}
	@media (pointer: coarse) {
		.handle {
			width: 1.5rem;
			height: 1.5rem;
		}
		.handle.width,
		.handle.size {
			right: -0.75rem;
		}
		.handle.size {
			bottom: -0.75rem;
		}
		.handle.rotate {
			top: -2.25rem;
		}
	}
	.hint {
		margin: 0;
	}
</style>
