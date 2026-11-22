<script lang="ts" module>
	export type CharacterViewId = 'sheet' | 'expressions' | '3d';
	export const CHARACTER_VIEWS: readonly CharacterViewId[] = ['sheet', 'expressions', '3d'];
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { freeOf, shell } from '$lib/shell/shell.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The character's own view, the workspace's stage when no tab took it (research/redesign/characters.html, A with C's
	 * split): Sheet, Expressions or 3D full-bleed under the bar, switched by the pill at the bottom (V); S splits the
	 * stage in two – the view beside the 3D place, or the sheet beside it – with the stage's divider and split (#765).
	 * Each box keeps its words clear of the panels: `--in-l` / `--in-r`.
	 */
	let {
		view = $bindable('sheet'),
		split = $bindable(false),
		sheet,
		expressions,
		place
	}: {
		view?: CharacterViewId;
		split?: boolean;
		sheet: Snippet;
		expressions: Snippet;
		place: Snippet;
	} = $props();

	let W = $state(1440);
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const r = (px: number) => `${px / rem()}rem`;
	const NAMES: Record<CharacterViewId, () => string> = { sheet: () => m['character.view_sheet'](), expressions: () => m['character.expressions'](), '3d': () => m['character.view_3d']() };
	const ICONS: Record<CharacterViewId, string> = { sheet: 'user', expressions: 'grid', '3d': 'box' };

	const shown = $derived<CharacterViewId[]>(split ? [view, view === '3d' ? 'sheet' : '3d'] : [view]);
	const free = $derived(freeOf(shell.floats, W));
	const boxes = $derived.by(() => {
		if (shown.length < 2) return [{ x: 0, w: W }];
		const sx = Math.round(W * shell.split);
		return [
			{ x: 0, w: sx },
			{ x: sx, w: W - sx }
		];
	});
	const clearOf = (b: { x: number; w: number }) => {
		const gap = rem();
		return { l: Math.max(1.5 * gap, free.left - b.x + gap), rt: Math.max(1.5 * gap, b.x + b.w - free.right + gap) };
	};
	/** a tab on the stage covers the view: its own pill and keys rest */
	const covered = $derived(shell.stage.length > 0);

	let dividing = $state(false);
	const typing = (e: KeyboardEvent) => {
		const t = e.target as HTMLElement | null;
		return !!t && (/^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName) || t.isContentEditable);
	};
	function onkeydown(e: KeyboardEvent) {
		if (covered || e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || typing(e) || document.querySelector('[aria-modal="true"]')) return;
		const key = e.key.toLowerCase();
		if (key === 's') split = !split;
		else if (key === 'v') view = CHARACTER_VIEWS[(CHARACTER_VIEWS.indexOf(view) + 1) % CHARACTER_VIEWS.length];
		else return;
		e.preventDefault();
	}
</script>

<svelte:window bind:innerWidth={W} {onkeydown} />

{#each shown as id, i (id)}
	{@const b = boxes[i]}
	{@const c = clearOf(b)}
	<div class="box" style:left={r(b.x)} style:width={r(b.w)} style:--in-l={r(c.l)} style:--in-r={r(c.rt)} data-view={id} data-testid="character-view-{id}">
		{#if id === 'sheet'}{@render sheet()}{:else if id === 'expressions'}{@render expressions()}{:else}{@render place()}{/if}
	</div>
{/each}
{#if shown.length === 2}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="divider"
		class:dragging={dividing}
		style:left={r(boxes[1].x)}
		title={m['shell.stage_divider']()}
		onpointerdown={(e) => ((dividing = true), (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId))}
		onpointermove={(e) => dividing && shell.setSplit(e.clientX / W)}
		onpointerup={() => (dividing = false)}
		onpointercancel={() => (dividing = false)}
		ondblclick={() => shell.setSplit(0.5)}
		data-testid="character-divider"
	></div>
{/if}
{#if !covered}
	<div class="pill" style:left={r((free.left + free.right) / 2)} role="group" aria-label={m['character.views']()} data-testid="character-views">
		{#each CHARACTER_VIEWS as id (id)}
			<button type="button" class:on={view === id} aria-pressed={view === id} onclick={() => (view = id)} title={m['character.view_keys']()} data-testid="character-view-pick-{id}"><Icon name={ICONS[id]} size="1rem" />{NAMES[id]()}</button>
		{/each}
		<span class="sep" aria-hidden="true"></span>
		<button type="button" class="round" class:on={split} aria-pressed={split} onclick={() => (split = !split)} aria-label={m['character.split']()} title={m['character.split']()} data-testid="character-split"><Icon name="columns" size="1rem" /></button>
	</div>
{/if}

<style>
	.box {
		position: fixed;
		top: 0;
		bottom: 0;
		overflow: hidden;
		background: var(--editor-stage);
		color: var(--editor-text);
	}
	.divider {
		position: fixed;
		z-index: 2;
		top: 0;
		bottom: 0;
		width: 0.8rem;
		margin-left: -0.4rem;
		display: flex;
		justify-content: center;
		cursor: col-resize;
		touch-action: none;
	}
	.divider::before {
		content: '';
		width: var(--editor-edge);
		height: 100%;
		background: var(--editor-line);
	}
	.divider:hover::before,
	.divider.dragging::before {
		background: var(--editor-accent);
	}
	.pill {
		position: fixed;
		z-index: 3;
		bottom: calc(var(--status-room) + 0.5rem);
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		gap: 0.2rem;
		min-height: var(--control-height);
		padding: 0 0.2rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1.2rem;
		background: var(--editor-panel-ground);
		text-shadow: var(--editor-halo);
		font-family: var(--editor-sans);
		white-space: nowrap;
	}
	.pill button {
		min-height: calc(var(--control-height) - 0.4rem);
		padding: 0 0.7rem;
		border: 0;
		border-radius: 1rem;
		background: transparent;
		color: inherit;
		gap: 0.35rem;
	}
	.pill button:hover {
		background: var(--editor-hover);
	}
	.pill button.on {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
		text-shadow: none;
	}
	.pill .round {
		width: calc(var(--control-height) - 0.4rem);
		padding: 0;
		border-radius: 50%;
	}
	.sep {
		width: var(--editor-edge);
		height: 1.2rem;
		margin: 0 0.15rem;
		background: var(--editor-line);
	}
</style>
