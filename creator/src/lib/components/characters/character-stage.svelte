<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { characterImage, lookImage, type CharacterLook, type CharacterView, type MoodVocabulary, type RegistryCharacter } from '@scutoons/content';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The character's Sheet view: one picture of the current persona › look › mood · view, big (the Expressions tab picks
	 * the mood and the view), in its box – a stage cell or a panel (`--in-l` / `--in-r` and the cell's `--stage-*` room).
	 * The item tab and the Inspector name what it shows. It zooms like a canvas: two fingers or Ctrl + wheel, drag to pan
	 * while zoomed, a double click fits again.
	 */
	let {
		character,
		persona,
		look,
		mood = 'neutral',
		view = 'portrait'
	}: {
		character: RegistryCharacter;
		persona: string;
		look: CharacterLook;
		moods: MoodVocabulary;
		mood?: string;
		view?: CharacterView;
	} = $props();

	const own = $derived(lookImage(look, mood, view));
	const src = $derived(characterImage(character, { persona, look: look.id, mood }, view, { isMissing: isKnownMissing }));

	// ── zoom: scale + offset; pinch / Ctrl + wheel at the pointer, pan while zoomed, double click fits ──
	let scale = $state(1);
	let x = $state(0);
	let y = $state(0);
	let box = $state<HTMLElement>();
	const pointers = new Map<number, { x: number; y: number }>();
	let last: { d: number; cx: number; cy: number } | null = null;
	function fit() {
		scale = 1;
		x = 0;
		y = 0;
	}
	$effect(() => {
		void mood;
		void view;
		void look.id;
		fit();
	});
	function zoomAt(cx: number, cy: number, factor: number) {
		const r = box?.getBoundingClientRect();
		if (!r) return;
		const next = Math.min(8, Math.max(1, scale * factor));
		const px = cx - r.left - r.width / 2;
		const py = cy - r.top - r.height / 2;
		x = px - ((px - x) * next) / scale;
		y = py - ((py - y) * next) / scale;
		scale = next;
		if (scale === 1) fit();
	}
	function wheel(e: WheelEvent) {
		if (!e.ctrlKey && !e.metaKey) return;
		e.preventDefault();
		zoomAt(e.clientX, e.clientY, Math.exp(-e.deltaY / 300));
	}
	function down(e: PointerEvent) {
		pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		last = null;
	}
	function move(e: PointerEvent) {
		const p = pointers.get(e.pointerId);
		if (!p) return;
		const before = { ...p };
		pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
		if (pointers.size === 2) {
			const [a, b] = [...pointers.values()];
			const d = Math.hypot(a.x - b.x, a.y - b.y);
			const cx = (a.x + b.x) / 2;
			const cy = (a.y + b.y) / 2;
			if (last) {
				zoomAt(cx, cy, d / last.d);
				x += cx - last.cx;
				y += cy - last.cy;
			}
			last = { d, cx, cy };
		} else if (pointers.size === 1 && scale > 1) {
			x += e.clientX - before.x;
			y += e.clientY - before.y;
		}
	}
	function up(e: PointerEvent) {
		pointers.delete(e.pointerId);
		last = null;
	}
</script>


<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="stage view-{view}" onwheel={wheel} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} ondblclick={fit} data-testid="character-stage">
	<div class="frame" bind:this={box}>
		{#if src}<img src={asset(src)} alt="" draggable="false" style:transform="translate({x}px, {y}px) scale({scale})" />{/if}
	</div>
	{#if !own}<span class="fallback" title={m['creator.characters_stage_fallback_hint']()}>{m['creator.characters_stage_fallback']()}</span>{/if}
	{#if scale > 1}<button type="button" class="sq fit" onclick={fit} title={m['creator.characters_stage_fit']()} aria-label={m['creator.characters_stage_fit']()}><Icon name="minimize" /></button>{/if}
</div>

<style>
	/* the whole box; the picture keeps to the cell's clear room */
	.stage {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: var(--tool-ground);
		touch-action: none;
		user-select: none;
	}
	.frame {
		position: absolute;
		top: calc(var(--stage-top, 0rem) + var(--control-height) + 1rem);
		bottom: calc(var(--stage-bottom, 0rem) + 0.75rem);
		left: var(--in-l, 1rem);
		right: var(--in-r, 1rem);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.frame img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
		transform-origin: center;
		pointer-events: none;
	}
	.view-full img,
	.view-portrait img {
		height: 100%;
	}
	.view-avatar img {
		max-width: min(24rem, 80%);
		max-height: 80%;
	}
	.fallback {
		position: absolute;
		left: var(--in-l, 1rem);
		max-width: calc(100% - var(--in-l, 1rem) - var(--in-r, 1rem));
		padding: 0.25rem 0.6rem;
		border-radius: var(--editor-radius);
		background: var(--tool-panel);
		text-shadow: var(--tool-halo);
		font-size: var(--editor-small);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		top: calc(var(--stage-top, 0rem) + 0.5rem);
		color: var(--tool-warn);
	}
	.sq {
		all: unset;
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		border-radius: 50%;
		cursor: pointer;
	}
	.sq:hover,
	.sq:focus-visible {
		background: var(--tool-hover);
	}
	.fit {
		position: absolute;
		right: var(--in-r, 1rem);
		top: calc(var(--stage-top, 0rem) + 0.5rem);
		background: var(--tool-panel);
		text-shadow: var(--tool-halo);
	}
</style>
