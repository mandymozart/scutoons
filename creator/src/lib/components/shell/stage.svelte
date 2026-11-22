<script lang="ts">
	import { tick, type Snippet } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { measureDrop, type Floats, type Rect } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { iconOf } from '$lib/shell/address.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { arriving, dropPlan, isCanvas, leaving, type Stage } from '$lib/shell/stage.ts';
	import { freeOf } from '$lib/shell/shell.svelte.ts';
	import { boxOf, fly, motionOn, type Box } from '$lib/shell/flight.ts';

	/**
	 * Taking the stage (research/redesign/stage.html): the tabs on the stage fill the whole window under the bar – a canvas
	 * edge to edge, a page with its ground under the bar and its content clear of the bar's row; the panels float over
	 * both, the stage keeps no room for them; two tabs
	 * share it with a hairline divider. The bottom pill holds the workspace's own view and the tabs on the stage. Every
	 * move flies between the panel and the stage. `content(tab)` draws a tab; `stage` = the tabs on it that have content.
	 */
	let { floats, stage, content, name }: { floats: Floats; stage: Stage; content: Snippet<[tab: string]>; name: (tab: string) => string } = $props();

	let W = $state(1440);
	let H = $state(900);
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const r = (px: number) => `${px / rem()}rem`;

	const holder = (tab: string) => floats.specs.find((s) => floats.tabsOf(s.id).includes(tab))?.id;
	/** a tab arriving on the stage is its panel's front tab, so that panel shows only its head */
	export function take(tab: string) {
		const p = holder(tab);
		if (p) {
			floats.reopen(p);
			floats.select(p, tab);
		}
	}

	// the room the panels leave free (client px): the pill sits in its middle
	const free = $derived(freeOf(floats, W));
	const boxes = $derived.by<Rect[]>(() => {
		if (stage.length < 2) return [{ x: 0, y: 0, w: W, h: H }];
		const sx = Math.round(W * shell.split);
		return [
			{ x: 0, y: 0, w: sx, h: H },
			{ x: sx, y: 0, w: W - sx, h: H }
		];
	});

	// ── the flight: measure, apply, measure again, fly ──
	const stageEl = (tab: string) => document.querySelector(`[data-stage-box="${tab}"]`);
	const panelEl = (tab: string) => {
		const p = holder(tab);
		const el = p ? document.querySelector(`[data-float="${p}"]`) : null;
		return el?.querySelector('[role="tabpanel"]') ?? el?.querySelector(`[data-tab="${tab}"]`) ?? el;
	};
	const viewEl = () => document.querySelector('[data-testid="stage"]');
	const pillEl = () => document.querySelector('[data-testid="stage-view"]');

	$effect(() => {
		shell.mover = async (apply: () => void, was: Stage, now: Stage) => {
			if (!motionOn()) return apply();
			const out = leaving(was, now);
			const into = arriving(was, now);
			const view = shell.front?.label ?? '';
			const froms: Array<[Box | null, string]> = [
				...out.map((t): [Box | null, string] => [boxOf(stageEl(t)), name(t)]),
				...into.map((t): [Box | null, string] => [boxOf(panelEl(t)), name(t)])
			];
			if (!was.length) froms.push([boxOf(viewEl()), view]);
			if (!now.length) froms.push([boxOf(pillEl()), view]);
			apply();
			await tick();
			const tos = [...out.map((t) => boxOf(panelEl(t))), ...into.map((t) => boxOf(stageEl(t)))];
			if (!was.length) tos.push(boxOf(pillEl()));
			if (!now.length) tos.push(boxOf(viewEl()));
			froms.forEach(([from, label], i) => {
				const to = tos[i];
				if (from && to) void fly(from, to, label);
			});
		};
		return () => (shell.mover = null);
	});

	// ── a tab dragged over the stage; a stage chip dragged back onto a panel ──
	export function zone(tab: string, x: number, y: number): { rect: Rect; label: string } | null {
		if (y < rem() * 3) return null;
		const plan = dropPlan(stage, tab, x / W);
		if (!plan) return null;
		const n = name(tab);
		if (plan.kind === 'take') return { rect: { x: 0, y: 0, w: W, h: H }, label: m['shell.stage_takes']({ tab: n }) };
		const half = plan.side === 'left' ? { x: 0, y: 0, w: W / 2, h: H } : { x: W / 2, y: 0, w: W / 2, h: H };
		if (plan.kind === 'split') return { rect: half, label: (plan.side === 'left' ? m['shell.stage_split_left'] : m['shell.stage_split_right'])({ tab: n }) };
		return { rect: boxes[plan.side === 'left' ? 0 : 1], label: m['shell.stage_replaces']({ tab: n, other: name(plan.out) }) };
	}
	export function drop(tab: string, x: number, y: number): boolean {
		if (!zone(tab, x, y)) return false;
		const plan = dropPlan(stage, tab, x / W);
		take(tab);
		if (plan?.kind === 'take') shell.send(tab);
		else if (plan) shell.splitIn(tab, plan.side);
		return true;
	}

	const pressChip = (e: PointerEvent, tab: string) => {
		if (e.button !== 0) return;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		floats.grabTab(tab, e.clientX, e.clientY);
	};
	const moveChip = (e: PointerEvent) => {
		if (!floats.heldTab) return;
		const { target, marker } = measureDrop(floats, e.clientX, e.clientY);
		floats.dragTab(e.clientX, e.clientY, target, marker);
	};
	const dropChip = (tab: string) => {
		const h = floats.heldTab;
		if (!h?.moved || !h.target) return floats.cancelTab();
		const into = h.target.panel;
		floats.dropTab();
		floats.reopen(into);
		floats.select(into, tab);
		shell.back(tab);
	};

	// ── the divider ──
	let dividing = $state(false);
	const pressDivider = (e: PointerEvent) => {
		dividing = true;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	};
	const moveDivider = (e: PointerEvent) => {
		if (dividing) shell.setSplit(e.clientX / W);
	};
</script>

<svelte:window bind:innerWidth={W} bind:innerHeight={H} />

{#if stage.length}
	{#each stage as tab, i (tab)}
		{@const b = boxes[i]}
		{#if isCanvas(tab)}
			<div class="box canvas" class:clear={shell.clear} style:left={r(b.x)} style:top={r(b.y)} style:width={r(b.w)} style:height={r(b.h)} data-stage-box={tab} data-testid="on-stage" data-tab={tab}>
				{@render content(tab)}
			</div>
		{:else}
			<div class="box page" class:clear={shell.clear} style:left={r(b.x)} style:top={r(b.y)} style:width={r(b.w)} style:height={r(b.h)} data-stage-box={tab} data-testid="on-stage" data-tab={tab}>
				<div class="page-body">
					<div class="page-col">{@render content(tab)}</div>
				</div>
			</div>
		{/if}
	{/each}
	{#if stage.length === 2}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="divider"
			class:dragging={dividing}
			style:left={r(boxes[1].x)}
			title={m['shell.stage_divider']()}
			onpointerdown={pressDivider}
			onpointermove={moveDivider}
			onpointerup={() => (dividing = false)}
			onpointercancel={() => (dividing = false)}
			ondblclick={() => shell.setSplit(0.5)}
			data-testid="stage-divider"
		></div>
	{/if}
	<div class="pill" style:left={r((free.left + free.right) / 2)} data-testid="stage-pill">
		<button type="button" onclick={() => shell.home()} title={m['shell.stage_view']()} data-testid="stage-view">
			<Icon name={shell.state.front ? iconOf(shell.state.front) : 'home'} size="1rem" />{shell.front?.label ?? ''}
		</button>
		{#each stage as tab (tab)}
			<span class="sep" aria-hidden="true"></span>
			<button
				type="button"
				class="chip"
				title={m['shell.stage_chip']({ tab: name(tab) })}
				onpointerdown={(e) => pressChip(e, tab)}
				onpointermove={moveChip}
				onpointerup={() => dropChip(tab)}
				onpointercancel={() => floats.cancelTab()}
				data-testid="stage-chip"><span class="dot" aria-hidden="true"></span>{name(tab)}</button
			>
			<button type="button" class="round" onclick={() => shell.back(tab)} aria-label={m['shell.main_back']()} title={m['shell.main_back']()} data-testid="stage-back">
				<Icon name="minimize-2" size="0.9rem" strokeWidth={2} />
			</button>
		{/each}
	</div>
{/if}

<style>
	/* a tab on the stage: the whole window (or its half), under the bar too; the page stays mounted under it */
	.box {
		position: fixed;
		z-index: 30;
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		overflow: hidden;
		color: var(--editor-text);
	}
	.box.canvas {
		background: var(--editor-stage);
	}
	.box.canvas > :global(.ws-tab) {
		flex: 1;
	}
	/* a canvas runs under the bar and the panels; its own controls read these to keep clear of them */
	.box.canvas {
		--stage-left: 1.5rem;
		--stage-right: 1.5rem;
		--stage-top: 0rem;
		--stage-bottom: calc(var(--status-room) + var(--control-height) + 1rem);
	}
	.box.canvas.clear {
		--stage-top: var(--shell-top);
	}
	.box.page {
		background: var(--editor-surface);
		overflow: auto;
	}
	.page-body {
		box-sizing: border-box;
		padding: 1rem 1.5rem 0;
		padding-bottom: calc(var(--status-room) + var(--control-height) + 1.5rem);
	}
	.box.page.clear .page-body {
		padding-top: calc(var(--shell-top) + 1rem);
	}
	.page-col {
		display: flex;
		flex-direction: column;
		max-width: 48rem;
		margin: 0 auto;
	}
	.divider {
		position: fixed;
		z-index: 31;
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
	/* the bottom pill: the workspace's own view, then the tabs on the stage */
	.pill {
		position: fixed;
		z-index: 45;
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
		gap: 0.35rem;
	}
	.pill button:hover {
		background: var(--editor-hover);
	}
	.pill .chip {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
		text-shadow: none;
		cursor: grab;
		touch-action: none;
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
	.dot {
		width: 0.45rem;
		height: 0.45rem;
		border-radius: 50%;
		background: var(--editor-accent);
	}
	:global(.stage-ghost) {
		position: fixed;
		z-index: 95;
		box-sizing: border-box;
		overflow: hidden;
		padding: 0.6rem 0.9rem;
		border-radius: var(--editor-radius);
		background: var(--editor-panel-ground);
		color: var(--editor-text);
		text-shadow: var(--editor-halo);
		box-shadow:
			0 0 0 1px var(--editor-accent),
			0 0.6rem 2rem rgb(0 0 0 / 0.45);
		font-family: var(--editor-sans);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
		pointer-events: none;
	}
</style>
