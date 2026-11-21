<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { characterImage, lookImage, moodLabel, type CharacterLook, type CharacterView, type MoodVocabulary, type RegistryCharacter } from '@scutoons/content';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The character workspace's background: one picture of the current persona › look › mood · view, big (the Moods panel
	 * picks the mood and the view). It zooms like a canvas: two fingers or Ctrl + wheel, drag to pan while zoomed, a double
	 * click fits again. `grid` swaps the picture for `gridContent` (every mood × view of the look to fill).
	 */
	let {
		character,
		persona,
		look,
		moods,
		locale,
		mood = 'neutral',
		view = 'portrait',
		grid = false,
		bust = 0,
		label,
		gridContent
	}: {
		character: RegistryCharacter;
		persona: string;
		look: CharacterLook;
		moods: MoodVocabulary;
		locale: string;
		mood?: string;
		view?: CharacterView;
		grid?: boolean;
		bust?: number;
		/** the path shown on the stage: "Regular › päpstlich" */
		label: string;
		gridContent: Snippet;
	} = $props();

	const VIEW_LABELS: Record<CharacterView, () => string> = {
		avatar: () => m['creator.characters_view_avatar'](),
		portrait: () => m['creator.characters_view_portrait'](),
		full: () => m['creator.characters_view_full']()
	};
	const own = $derived(lookImage(look, mood, view));
	const src = $derived(characterImage(character, { persona, look: look.id, mood }, view, { isMissing: isKnownMissing }));
	const busted = (path: string) => (path ? `${asset(path)}${bust ? `?v=${bust}` : ''}` : '');

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

<div class="stage-wrap">
	{#if grid}
		<div class="grid-area">{@render gridContent()}</div>
	{:else}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="stage view-{view}" bind:this={box} onwheel={wheel} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} ondblclick={fit} data-testid="character-stage">
			{#if src}<img src={busted(src)} alt="" draggable="false" style:transform="translate({x}px, {y}px) scale({scale})" />{/if}
			<span class="where" data-testid="stage-where">{label} › {moodLabel(mood, locale, moods)} · {VIEW_LABELS[view]()}</span>
			{#if !own}<span class="fallback" title={m['creator.characters_stage_fallback_hint']()}>{m['creator.characters_stage_fallback']()}</span>{/if}
			{#if scale > 1}<button type="button" class="sq fit" onclick={fit} title={m['creator.characters_stage_fit']()} aria-label={m['creator.characters_stage_fit']()}><Icon name="minimize" /></button>{/if}
		</div>
	{/if}
</div>

<style>
	.stage-wrap {
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
		flex: 1;
	}
	.where {
		position: absolute;
		left: 1rem;
		top: 1rem;
		max-width: calc(100% - 2rem);
		padding: 0.25rem 0.6rem;
		border-radius: var(--editor-radius);
		background: var(--editor-panel-ground, var(--editor-surface));
		color: var(--editor-text);
		font-size: var(--editor-small);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
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
		background: var(--editor-hover);
	}
	.stage {
		position: relative;
		flex: 1;
		min-height: 12rem;
		overflow: hidden;
		background: var(--editor-stage);
		display: flex;
		align-items: center;
		justify-content: center;
		touch-action: none;
		user-select: none;
	}
	.stage img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
		transform-origin: center;
		pointer-events: none;
	}
	.stage.view-full img,
	.stage.view-portrait img {
		height: 100%;
	}
	.stage.view-avatar img {
		max-width: min(24rem, 80%);
		max-height: 80%;
	}
	.fallback {
		position: absolute;
		left: 1rem;
		top: 3rem;
		padding: 0.25rem 0.6rem;
		border-radius: var(--editor-radius);
		background: var(--editor-panel-ground, var(--editor-surface));
		color: var(--editor-warn);
		font-size: var(--editor-small);
	}
	.fit {
		position: absolute;
		right: 1rem;
		top: 1rem;
		background: var(--editor-panel-ground, var(--editor-surface));
	}
	.grid-area {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 1rem;
		background: var(--editor-stage);
	}
</style>
