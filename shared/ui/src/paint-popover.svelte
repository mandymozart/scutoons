<script lang="ts">
	import ColourPopover from './colour-popover.svelte';
	import ScrubNumber from './scrub-number.svelte';
	import Segmented from './segmented.svelte';
	import Icon from './icon.svelte';
	import { uiLabels } from './ui-labels.ts';
	import { colourAt, isGradient, paintKind, stopBarCss, toKind, type Gradient, type GradientStop, type Paint, type PaintKind } from './paint.ts';

	/**
	 * The paint popover (issue #94): Figma's paint switch on top – Solid · Linear · Radial as icons (owner 2026-10-20: a square filled with a bw gradient;
	 * grouped, square).
	 * Solid = the colour popover as before. A gradient: the stop bar (its stops left to right; drag a stop to move it,
	 * click the bar to add one with the colour there, the selected stop has a thicker frame), the selected stop's
	 * position and a remove button (a gradient keeps two stops at least), the colour popover for the selected stop,
	 * and the gradient's own controls – Linear: the angle; Radial: the centre (x, y) and the radius, in % of the box.
	 */
	let {
		value,
		fallback,
		palette = [],
		onpick,
		oneyedropper,
		eyedropperLabel
	}: {
		value: Paint | null | undefined;
		fallback?: string | null;
		palette?: { name: string; hex: string }[];
		onpick: (paint: Paint) => void;
		oneyedropper?: () => void;
		eyedropperLabel?: string;
	} = $props();

	const t = uiLabels();
	const kind = $derived(paintKind(value));
	const gradient = $derived(isGradient(value) ? value : null);
	let selected = $state(0);
	/** the stop being dragged (its index), -1 = none */
	let dragging = -1;
	const stop = $derived(gradient ? gradient.stops[Math.min(selected, gradient.stops.length - 1)] : null);

	function setKind(k: PaintKind) {
		selected = 0;
		onpick(toKind(value, k, fallback ?? '#000000'));
	}
	function update(next: Partial<Gradient>) {
		if (gradient) onpick({ ...gradient, ...next } as Gradient);
	}
	/** stops are stored in ascending order (the validator wants it): sort, and keep the selection (and a drag) on `focus` */
	function commit(stops: GradientStop[], focus: GradientStop) {
		const sorted = [...stops].sort((a, b) => a.at - b.at);
		const i = sorted.indexOf(focus);
		if (dragging >= 0) dragging = i;
		selected = i;
		update({ stops: sorted });
	}
	function setStop(i: number, next: Partial<GradientStop>) {
		if (!gradient) return;
		const changed = { ...gradient.stops[i], ...next };
		commit(gradient.stops.map((s, k) => (k === i ? changed : s)), changed);
	}

	// ── the stop bar ──
	let bar: HTMLDivElement | undefined = $state();
	const atOf = (e: PointerEvent) => {
		const r = bar!.getBoundingClientRect();
		return Math.round(Math.min(100, Math.max(0, ((e.clientX - r.left) / r.width) * 100)));
	};
	function barDown(e: PointerEvent) {
		if (!gradient || e.button !== 0 || (e.target as HTMLElement).closest('.stop')) return;
		const at = atOf(e);
		const added = { color: colourAt(gradient.stops, at), at };
		commit([...gradient.stops, added], added);
	}
	function stopDown(e: PointerEvent, i: number) {
		if (e.button !== 0) return;
		e.preventDefault();
		selected = i;
		dragging = i;
		try {
			(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		} catch {
			/* synthetic */
		}
	}
	function stopKey(e: KeyboardEvent, i: number) {
		const d = (e.key === 'ArrowLeft' ? -1 : e.key === 'ArrowRight' ? 1 : 0) * (e.shiftKey ? 10 : 1);
		if (d && gradient) {
			e.preventDefault();
			setStop(i, { at: Math.min(100, Math.max(0, gradient.stops[i].at + d)) });
		}
		if ((e.key === 'Delete' || e.key === 'Backspace') && gradient && gradient.stops.length > 2) {
			e.preventDefault();
			remove(i);
		}
	}
	function remove(i: number) {
		if (!gradient || gradient.stops.length <= 2) return;
		update({ stops: gradient.stops.filter((_, k) => k !== i) });
		selected = Math.max(0, i - 1);
	}
</script>

<div class="sc-paint-popover">
	<Segmented
		label={t('paint')}
		value={kind}
		onchange={setKind}
		square
		options={[
			{ value: 'solid', icon: 'paint-solid', title: t('paintSolid') },
			{ value: 'linear', icon: 'paint-linear', title: t('paintLinear') },
			{ value: 'radial', icon: 'paint-radial', title: t('paintRadial') }
		]}
		testid="paint-kind"
	/>

	{#if gradient && stop}
		<div
			bind:this={bar}
			class="bar"
			role="group"
			aria-label={t('gradientStops')}
			style:--bar={stopBarCss(gradient.stops)}
			onpointerdown={barDown}
			data-testid="paint-stops"
		>
			{#each gradient.stops as s, i (i)}
				<button
					type="button"
					class="stop"
					class:is-selected={i === Math.min(selected, gradient.stops.length - 1)}
					style:left="{s.at}%"
					style:--c={s.color}
					aria-label="{t('gradientStop')} {i + 1}: {s.color} · {s.at} %"
					title="{s.color} · {s.at} %"
					onpointerdown={(e) => stopDown(e, i)}
					onpointermove={(e) => dragging === i && setStop(i, { at: atOf(e) })}
					onpointerup={() => (dragging = -1)}
					onpointercancel={() => (dragging = -1)}
					onkeydown={(e) => stopKey(e, i)}
				></button>
			{/each}
		</div>
		<div class="row">
			<ScrubNumber icon="sliders" label={t('stopPosition')} unit="%" min={0} max={100} step={1} value={stop.at} onchange={(v) => setStop(selected, { at: v ?? 0 })} testid="paint-stop-at" />
			<button type="button" class="remove" disabled={gradient.stops.length <= 2} aria-label={t('removeStop')} title={t('removeStop')} onclick={() => remove(selected)}>
				<Icon name="trash" size="1.1em" />
			</button>
		</div>
		{#if gradient.type === 'linear'}
			<div class="row">
				<ScrubNumber icon="rotate-cw" label={t('gradientAngle')} unit="°" step={1} value={gradient.angle} onchange={(v) => update({ angle: v ?? 180 })} testid="paint-angle" />
			</div>
		{:else}
			<div class="row">
				<ScrubNumber tag="X" label={t('gradientCentreX')} unit="%" step={1} value={gradient.center.x} onchange={(v) => update({ center: { ...gradient.center, x: v ?? 50 } })} testid="paint-cx" />
				<ScrubNumber tag="Y" label={t('gradientCentreY')} unit="%" step={1} value={gradient.center.y} onchange={(v) => update({ center: { ...gradient.center, y: v ?? 50 } })} testid="paint-cy" />
			</div>
			<div class="row">
				<ScrubNumber icon="circle" label={t('gradientRadius')} unit="%" min={1} step={1} value={gradient.radius} onchange={(v) => update({ radius: v ?? 100 })} testid="paint-radius" />
			</div>
		{/if}
		<ColourPopover value={stop.color} {fallback} {palette} onpick={(hex) => setStop(Math.min(selected, gradient.stops.length - 1), { color: hex })} {oneyedropper} {eyedropperLabel} />
	{:else}
		<ColourPopover value={typeof value === 'string' ? value : null} {fallback} {palette} onpick={(hex) => onpick(hex)} {oneyedropper} {eyedropperLabel} />
	{/if}
</div>

<style>
	.sc-paint-popover {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		box-sizing: border-box;
		width: 16rem;
		padding: var(--space-05);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
	}
	/* the colour popover inside: no second frame */
	.sc-paint-popover :global(.sc-colour-popover) {
		width: auto;
		padding: 0;
		border: 0;
	}
	/* the stop bar: the gradient left to right over a checkerboard, the stops as small squares on it */
	.bar {
		position: relative;
		height: 1.5rem;
		margin: 0.25rem 0.625rem; /* room for the stops, which stand out of the bar */
		border: var(--border-width-regular) solid var(--ink);
		background:
			var(--bar),
			repeating-conic-gradient(var(--border-muted) 0 25%, var(--paper) 0 50%) 0 0 / 0.5rem 0.5rem;
		cursor: copy;
		touch-action: none;
	}
	.stop {
		position: absolute;
		top: 50%;
		box-sizing: border-box;
		width: 1rem;
		height: 1.75rem;
		margin: 0;
		padding: var(--border-width-hairline);
		transform: translate(-50%, -50%);
		border: var(--border-width-regular) solid var(--ink);
		background: linear-gradient(var(--c), var(--c)) content-box, var(--paper);
		cursor: ew-resize;
		touch-action: none;
	}
	/* a finger: wider stops */
	@media (pointer: coarse) {
		.stop {
			width: 1.25rem;
		}
	}
	.stop.is-selected {
		border-width: calc(2 * var(--border-width-regular));
	}
	.row {
		display: flex;
		gap: var(--space-025);
	}
	.row > :global(*) {
		flex: 1;
		min-width: 0;
	}
	/* remove: a square beside the position, grouped (DESIGN.md "Round vs square") */
	.remove {
		flex: none !important;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		margin: 0;
		padding: 0;
		border: var(--border-width-regular) solid var(--border-muted);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
	}
	.remove:hover:not(:disabled) {
		border-color: var(--ink);
	}
	.remove:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
