<script lang="ts">
	import { onMount } from 'svelte';
	import { TextBoxEffect, evaluateText, type Dialog, type DialogFont, type DialogVariant } from '@scutoons/content';
	import type { DialogPlay, TextExpressions } from '../types.ts';
	import { wobblyPath } from './wobbly.ts';
	import EffectNone from '../effects/effect-none.svelte';
	import EffectShiver from '../effects/effect-shiver.svelte';
	import EffectPulse from '../effects/effect-pulse.svelte';
	import EffectWobble from '../effects/effect-wobble.svelte';
	import EffectEnter from '../effects/effect-enter.svelte';

	/**
	 * One text box. `play` = a showTextBox event is revealing it right now (`params.dialog`): the box appears with the
	 * event's effect / delay / duration and stays. `expressions`: its `{…}` are evaluated for the reader (live: a value
	 * that changes re-renders the text) in the text's language `lang`; none = the text as written.
	 */
	let {
		dialog,
		index,
		play,
		expressions,
		lang
	}: {
		dialog: Dialog;
		/** its place in the slice's `dialog[]` (`data-index`: the creator measures the drawn boxes by it) */
		index?: number;
		play?: DialogPlay | null;
		expressions?: TextExpressions;
		lang?: string;
	} = $props();

	const EFFECTS: Partial<Record<TextBoxEffect, typeof EffectNone>> = {
		[TextBoxEffect.NONE]: EffectNone,
		[TextBoxEffect.SHIVER]: EffectShiver,
		[TextBoxEffect.PULSE]: EffectPulse,
		[TextBoxEffect.WOBBLE]: EffectWobble
	};
	/** the entrances (agents/FX.md): one component, the kind decides */
	const ENTRANCES = [TextBoxEffect.POP, TextBoxEffect.SLAM, TextBoxEffect.FADE, TextBoxEffect.TYPE_ON] as const;
	type Entrance = (typeof ENTRANCES)[number];
	const Effect = $derived(play ? (EFFECTS[play.effect ?? TextBoxEffect.NONE] ?? EffectNone) : null);

	/** Smallest scale the auto-fit pass may shrink the font to before it lets the text overflow. */
	const MIN_FIT = 0.8;
	const FIT_STEP = 0.025;

	const variant: DialogVariant = $derived(dialog.variant ?? 'bubble');
	const boxed = $derived(variant === 'caption' || variant === 'narration');
	/** caption / narration draw their paper box + ink border unless `frame: false` */
	const framed = $derived(boxed && dialog.frame !== false);
	/** a drawn speech bubble: only with `frame: true` (the bubbles of chapter 1/2 are in the art) */
	const bubble = $derived(variant === 'bubble' && dialog.frame === true);
	/** the drawn bubble's outline: a box with a tail (speech – an older 'oval' is drawn as a box too) or a wobbly edge (shouting, singing) */
	const shape = $derived(bubble ? (dialog.shape === 'wobbly' ? 'wobbly' : 'box') : null);
	const tail = $derived(bubble ? (dialog.tail ?? (shape === 'wobbly' ? 'none' : 'bottom-left')) : 'none');
	const stroked = $derived(!!dialog.stroke);
	const font: DialogFont = $derived(dialog.font ?? (boxed ? 'mono' : 'lettering'));
	const align = $derived(dialog.align ?? (boxed ? 'left' : 'center'));
	const valign = $derived(dialog.valign ?? 'middle');
	const written = $derived(Array.isArray(dialog.text) ? dialog.text : [dialog.text]);
	/** a line's identity for the reader's visits (sequences, cycles, once-only) */
	const textId = (index: number) => `${dialog.id ?? ''}/${index}`;
	const evaluated = $derived(
		written.map((line, index) => {
			const resolver = expressions?.resolver(textId(index), lang);
			return resolver ? evaluateText(line, resolver) : { text: String(line ?? ''), visited: [] as number[] };
		})
	);
	const lines = $derived(evaluated.map((line) => line.text));
	// the lists a line showed: the app counts the visit (once per page view)
	$effect(() => {
		evaluated.forEach((line, index) => {
			if (line.visited.length) expressions?.shown?.(textId(index), line.visited);
		});
	});
	const hasHeight = $derived(dialog.rect.height !== undefined);
	const maskColor = $derived(dialog.mask === true ? '#ffffff' : dialog.mask || null);

	let root: HTMLDivElement | null = $state(null);
	let box: HTMLDivElement | null = $state(null);
	let content: HTMLDivElement | null = $state(null);
	/** the box's laid-out size (px): the wobbly outline is drawn for it */
	let boxSize = $state<{ w: number; h: number } | null>(null);
	const wobbly = $derived(shape === 'wobbly' && boxSize ? { d: wobblyPath(boxSize.w, boxSize.h, dialog.id ?? lines.join(' ')) } : null);

	function overflows(): boolean {
		if (!box || !content) return false;
		const style = getComputedStyle(box);
		const available =
			box.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
		return (
			content.scrollHeight > available + 0.5 || content.scrollWidth > content.clientWidth + 0.5
		);
	}

	/** Content size right after the last fit pass, so its own font change doesn't re-trigger the observer. */
	let fittedSize = '';

	const sizeOf = (element: HTMLElement) => `${element.offsetWidth}x${element.offsetHeight}`;

	/** Shrink the font (down to MIN_FIT) until the text fits a box with a fixed height. */
	function fit() {
		if (!root || !box || !content) return;
		// always measure from full size
		let scale = 1;
		root.style.setProperty('--fit', '1');
		// not laid out yet (hidden, container width 0, image not loaded) → wait for the observer
		const measurable = hasHeight && box.clientWidth > 0 && box.clientHeight > 0 && content.clientWidth > 0;
		if (measurable) {
			while (scale > MIN_FIT && overflows()) {
				scale = Math.max(MIN_FIT, Math.round((scale - FIT_STEP) * 1000) / 1000);
				root.style.setProperty('--fit', String(scale));
			}
		}
		fittedSize = sizeOf(content);
		if (shape === 'wobbly' && (boxSize?.w !== box.offsetWidth || boxSize?.h !== box.offsetHeight)) {
			boxSize = { w: box.offsetWidth, h: box.offsetHeight };
		}
	}

	// re-fit whenever the text or geometry changes (e.g. locale switch)
	$effect(() => {
		void [lines, dialog.size, dialog.rect.width, dialog.rect.height, font, variant, framed, bubble, shape, box];
		fit();
	});

	onMount(() => {
		let pending = 0;
		const schedule = () => {
			cancelAnimationFrame(pending);
			pending = requestAnimationFrame(fit);
		};
		const observer = new ResizeObserver((entries) => {
			// ignore the content resize caused by our own --fit change
			const external = entries.some(
				(entry) => entry.target !== content || sizeOf(content!) !== fittedSize
			);
			if (external) schedule();
		});
		// slice (image load, column width), box (geometry) and content (web font swap)
		const slice = root?.closest('.slice');
		if (slice) observer.observe(slice);
		if (box) observer.observe(box);
		if (content) observer.observe(content);
		// fonts load lazily on first use, so `ready` can resolve before the lettering font is even requested
		document.fonts?.ready.then(schedule);
		document.fonts?.addEventListener('loadingdone', schedule);
		return () => {
			cancelAnimationFrame(pending);
			observer.disconnect();
			document.fonts?.removeEventListener('loadingdone', schedule);
		};
	});
</script>

<div
	bind:this={root}
	class="dialog dialog--{variant} font-{font}"
	class:has-height={hasHeight}
	class:framed
	class:bubble
	class:wobbly={shape === 'wobbly'}
	class:stroked
	class:uppercase={dialog.uppercase ?? false}
	data-id={dialog.id}
	data-index={index}
	style:left="{dialog.rect.x}%"
	style:top="{dialog.rect.y}%"
	style:width="{dialog.rect.width}%"
	style:height={hasHeight ? `${dialog.rect.height}%` : null}
	style:--size={dialog.size ?? 4}
	style:--pad={dialog.pad ?? 0.5}
	style:--rotation="{dialog.rotation ?? 0}deg"
	style:--scale={dialog.scale ?? 1}
	style:--dialog-color={dialog.color ?? null}
	style:--dialog-weight={dialog.weight ?? null}
	style:--dialog-stroke={stroked ? dialog.stroke : null}
	style:--dialog-stroke-width={stroked && dialog.strokeWidth !== undefined ? `${dialog.strokeWidth}em` : null}
	style:letter-spacing={dialog.letterSpacing ? `${dialog.letterSpacing}em` : null}
>
	{#if play && Effect}
		{#key play.key}
			{#if ENTRANCES.includes(play.effect as Entrance)}
				<EffectEnter kind={play.effect as Entrance} duration={play.duration} delay={play.delay} sound={play.sound} shake={play.shake} onComplete={() => play?.onComplete()}>
					{@render body()}
				</EffectEnter>
			{:else}
				<Effect duration={play.duration} delay={play.delay} sound={play.sound} onComplete={() => play?.onComplete()}>
					{@render body()}
				</Effect>
			{/if}
		{/key}
	{:else}
		{@render body()}
	{/if}
</div>

{#snippet body()}
	{#if maskColor}
		<div
			class="mask"
			class:ellipse={dialog.maskShape === 'ellipse'}
			style:background={maskColor}
			aria-hidden="true"
		></div>
	{/if}
	<div
		bind:this={box}
		class="box"
		style:justify-content={valign === 'top' ? 'flex-start' : valign === 'bottom' ? 'flex-end' : 'center'}
	>
		{#if wobbly}
			<svg class="outline" viewBox="0 0 {boxSize?.w} {boxSize?.h}" preserveAspectRatio="none" aria-hidden="true">
				<path d={wobbly.d} />
			</svg>
		{/if}
		{#if tail !== 'none'}
			<svg class="tail tail--{tail}" viewBox="0 0 16 14" preserveAspectRatio="none" aria-hidden="true">
				<path d={tail.endsWith('left') ? 'M3 0 L0 14 L13 0' : tail.endsWith('right') ? 'M3 0 L16 14 L13 0' : 'M2 0 L8 14 L14 0'} />
			</svg>
		{/if}
		<div bind:this={content} class="content" style:text-align={align}>
			{#each lines as line, index (index)}
				<span class="line">{line}</span>
			{/each}
		</div>
	</div>
{/snippet}

<style>
	.dialog {
		--ink: #000000;
		--paper: #ffffff;
		--fit: 1;
		position: absolute;
		box-sizing: border-box;
		isolation: isolate;
		/* around the centre: rotation, then a uniform scale of the laid-out box (text keeps its wrapping) */
		transform: rotate(var(--rotation)) scale(var(--scale, 1));
		pointer-events: none;
		color: var(--dialog-color, var(--ink));
		font-weight: var(--dialog-weight, 400);
		/* size is % of the slice width; .slice is the inline-size container */
		font-size: calc(var(--size) * var(--fit) * 1cqw);
		line-height: 1.15;
		overflow-wrap: anywhere;
		hyphens: auto;
		-webkit-hyphens: auto;
		white-space: pre-line;
	}

	.font-lettering {
		font-family: var(--font-lettering);
		line-height: 1.1;
		letter-spacing: 0.03em;
	}

	.font-mono {
		font-family: var(--font-mono);
		/* chapter 1 captions are set at ~1.35 */
		line-height: 1.35;
	}

	.uppercase {
		text-transform: uppercase;
	}

	.mask {
		position: absolute;
		inset: calc(var(--pad) * -1cqw);
		z-index: -1;
	}

	.mask.ellipse {
		border-radius: 50%;
	}

	.box {
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		width: 100%;
	}

	.has-height .box {
		height: 100%;
	}

	.content {
		min-width: 0;
	}

	.line {
		display: block;
	}

	/* caption / narration: on-brand panel caption (paper box, 2px ink border, square), unless frame: false */
	.framed .box {
		background: var(--paper);
		border: 2px solid var(--ink);
		padding: 0.8em 1em;
	}

	.dialog--narration {
		font-style: italic;
	}

	/* bubble + frame: a drawn speech bubble for text-free art – a paper box with an ink outline and a tail, like the
	   narration box (owner 2026-10-10: no ovals – an ellipse cut its text at the sides) */
	.bubble .box {
		position: relative;
		background: var(--paper);
		border: 2px solid var(--ink);
		padding: 0.8em 1em;
	}

	.has-height.bubble .box {
		height: 100%;
	}

	/* wobbly (shout / song): no box – the trembling outline is an SVG drawn for the box's size */
	.bubble.wobbly .box {
		border: 0;
		border-radius: 0;
		background: none;
		padding: 0.8em 1.1em;
	}

	.outline {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		z-index: -1;
		overflow: visible;
	}

	.outline path {
		fill: var(--paper);
		stroke: var(--ink);
		stroke-width: 2.5px;
		vector-effect: non-scaling-stroke;
		stroke-linejoin: round;
	}

	/* the tail starts inside the wobbly edge, so its paper covers the outline where it joins */
	.wobbly .tail {
		top: calc(100% - 7px);
	}

	.wobbly .tail--top-left,
	.wobbly .tail--top,
	.wobbly .tail--top-right {
		top: auto;
		bottom: calc(100% - 7px);
	}

	.tail {
		position: absolute;
		width: 1.1em;
		height: 1em;
		overflow: visible;
		/* over the outline: the tail's paper covers the border where it joins */
		top: calc(100% - 2px);
	}

	.tail path {
		fill: var(--paper);
		stroke: var(--ink);
		stroke-width: 2px;
		vector-effect: non-scaling-stroke;
		stroke-linejoin: miter;
	}

	.tail--bottom-left,
	.tail--top-left {
		left: 22%;
	}

	.tail--bottom,
	.tail--top {
		left: calc(50% - 0.55em);
	}

	.tail--bottom-right,
	.tail--top-right {
		right: 22%;
	}

	.tail--top-left,
	.tail--top,
	.tail--top-right {
		top: auto;
		bottom: calc(100% - 2px);
		transform: scaleY(-1);
	}

	/* an outline around the letters (e.g. an onomatopoeia: white letters, ink outline) */
	.stroked {
		-webkit-text-stroke: var(--dialog-stroke-width, 0.14em) var(--dialog-stroke);
		paint-order: stroke fill;
	}

</style>
