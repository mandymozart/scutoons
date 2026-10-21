<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { characterImage, lookImage, lookMoods, moodLabel, type CharacterLook, type CharacterView, type MoodVocabulary, type RegistryCharacter } from '@scutoons/content';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { edgeFade } from '$lib/editor/edge-fade.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The character page's stage (research/CHARACTER-PAGE.md E): one picture of the current persona › look › mood ·
	 * view, big; the views as three icon squares (face · bust · figure – owner 2026-10-20: "maybe create some symbols");
	 * the moods as a strip under it (edge to edge, the fade where it goes on). It zooms like a canvas (owner: "i hope our
	 * stage is finger zoomable"): two fingers or Ctrl + wheel, drag to pan while zoomed, a double click / tap fits again.
	 * The grid square swaps the picture for `grid` (every mood × view to fill – the look's ExpressionGrid).
	 */
	let {
		character,
		persona,
		look,
		moods,
		locale,
		mood = $bindable('neutral'),
		view = $bindable<CharacterView>('portrait'),
		grid = $bindable(false),
		bust = 0,
		label,
		gridContent,
		lead,
		tools
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
		/** the start of the bar (the look row: views sit at the end of it – CHARACTER-PAGE E round 3) */
		lead?: Snippet;
		/** squares at the end of the views (the page's own: Import, the Kanon square later) */
		tools?: Snippet;
	} = $props();

	const VIEWS: { id: CharacterView; icon: string; label: () => string }[] = [
		{ id: 'avatar', icon: 'view-avatar', label: () => m['creator.characters_view_avatar']() },
		{ id: 'portrait', icon: 'view-portrait', label: () => m['creator.characters_view_portrait']() },
		{ id: 'full', icon: 'view-full', label: () => m['creator.characters_view_full']() }
	];
	const strip = $derived(lookMoods(look, moods));
	const own = $derived(lookImage(look, mood, view));
	const src = $derived(characterImage(character, { persona, look: look.id, mood }, view, { isMissing: isKnownMissing }));
	const thumb = (x: string) => characterImage(character, { persona, look: look.id, mood: x }, 'avatar', { isMissing: isKnownMissing });
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
	<div class="stage-bar">
		<span class="grow">{@render lead?.()}</span>
		<span class="views" role="group" aria-label={m['creator.characters_view']()}>
			{#each VIEWS as v (v.id)}
				<button type="button" class="sq" aria-pressed={!grid && view === v.id} title={v.label()} aria-label={v.label()} onclick={() => ((view = v.id), (grid = false))} data-testid="stage-view-{v.id}"><Icon name={v.icon} /></button>
			{/each}
			<button type="button" class="sq" aria-pressed={grid} title={m['creator.characters_all_moods']()} aria-label={m['creator.characters_all_moods']()} onclick={() => (grid = !grid)} data-testid="stage-grid"><Icon name="grid" /></button>
		</span>
		{@render tools?.()}
	</div>
	{#if grid}
		<div class="grid-area">{@render gridContent()}</div>
	{:else}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="stage view-{view}" bind:this={box} onwheel={wheel} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} ondblclick={fit} data-testid="character-stage">
			{#if src}<img src={busted(src)} alt="" draggable="false" style:transform="translate({x}px, {y}px) scale({scale})" />{/if}
			<span class="where" data-testid="stage-where">{label} › {moodLabel(mood, locale, moods)} · {VIEWS.find((v) => v.id === view)?.label()}</span>
			{#if !own}<span class="fallback" title={m['creator.characters_stage_fallback_hint']()}>{m['creator.characters_stage_fallback']()}</span>{/if}
			{#if scale > 1}<button type="button" class="sq fit" onclick={fit} title={m['creator.characters_stage_fit']()} aria-label={m['creator.characters_stage_fit']()}><Icon name="minimize" /></button>{/if}
		</div>
	{/if}
	<div class="strip" use:edgeFade role="listbox" aria-label={m['creator.characters_moods']()} data-testid="mood-strip">
		{#each strip as x (x)}
			<button type="button" class="mood" class:on={x === mood} role="option" aria-selected={x === mood} onclick={() => ((mood = x), (grid = false))} data-control-exempt="a mood thumbnail with its word" data-testid="strip-{x}">
				<span class="pic">{#if thumb(x)}<img src={busted(thumb(x))} alt="" loading="lazy" draggable="false" />{/if}</span>
				<span class="word">{moodLabel(x, locale, moods)}</span>
			</button>
		{/each}
	</div>
</div>

<style>
	.stage-wrap {
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
		flex: 1;
	}
	.stage-bar {
		display: flex;
		align-items: center;
		gap: var(--space-025);
		padding: 0 var(--space-05) 0 0;
		min-height: var(--control-height);
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.where {
		position: absolute;
		left: var(--space-1);
		top: var(--space-1);
		max-width: calc(100% - 2 * var(--space-1));
		padding: 0 var(--space-05);
		background: var(--paper);
		font-size: var(--type-caption-size);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
	}
	.grow {
		flex: 1;
		min-width: 0;
		display: flex;
	}
	.views {
		display: inline-flex;
		flex: none;
	}
	/* borderless icon squares (CREATOR-NAV D): hover = the outline, on = inverted */
	.sq {
		all: unset;
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		cursor: pointer;
	}
	.sq:hover,
	.sq:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.sq[aria-pressed='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	.stage {
		position: relative;
		flex: 1;
		min-height: 12rem;
		overflow: hidden;
		background: var(--surface-hover);
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
		left: var(--space-1);
		bottom: var(--space-1);
		padding: 0 var(--space-05);
		font-size: var(--type-caption-size);
		background: var(--state-warning);
	}
	.fit {
		position: absolute;
		right: var(--space-05);
		top: var(--space-05);
		background: var(--paper);
	}
	.grid-area {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: var(--space-1);
	}
	.strip {
		display: flex;
		gap: var(--space-025);
		overflow-x: auto;
		scrollbar-width: none;
		padding: var(--space-05) var(--space-1);
		border-top: var(--border-width-hairline) solid var(--border-subtle);
		flex: none;
		--fade: 2.5rem;
	}
	.strip::-webkit-scrollbar {
		display: none;
	}
	.strip:global([data-fade='r']) {
		mask-image: linear-gradient(to right, #000 calc(100% - var(--fade)), transparent);
	}
	.strip:global([data-fade='l']) {
		mask-image: linear-gradient(to right, transparent, #000 var(--fade));
	}
	.strip:global([data-fade='lr']) {
		mask-image: linear-gradient(to right, transparent, #000 var(--fade), #000 calc(100% - var(--fade)), transparent);
	}
	.mood {
		all: unset;
		box-sizing: border-box;
		flex: none;
		width: 4.5rem;
		display: flex;
		flex-direction: column;
		cursor: pointer;
	}
	.mood .pic {
		display: block;
		aspect-ratio: 1;
		background: var(--surface-hover);
		overflow: hidden;
	}
	.mood img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.mood.on .pic {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.mood:focus-visible .pic {
		outline: var(--border-width-focus) solid var(--ink);
	}
	.word {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	@media (pointer: coarse) {
		.sq {
			width: max(2.75rem, var(--control-height));
			height: max(2.75rem, var(--control-height));
		}
	}
</style>
