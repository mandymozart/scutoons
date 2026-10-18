<script lang="ts">
	import { onMount } from 'svelte';
	import { TextBoxEffect, anchorTranslate, dialogFit, dialogTail, evaluateText, type Dialog, type DialogFont, type DialogVariant } from '@scutoons/content';
	import type { DialogPlay, TextExpressions } from '../types.ts';
	import { wobblyPath } from './wobbly.ts';
	import { measureTail, type TailDrawing } from './tail.ts';
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
	const boxed = $derived(variant === 'caption' || variant === 'narration' || variant === 'lyrics');
	/** caption / narration / lyrics draw their paper box + ink border unless `frame: false` */
	const framed = $derived(boxed && dialog.frame !== false);
	/** a drawn speech bubble: only with `frame: true` (the bubbles of chapter 1/2 are in the art) */
	const bubble = $derived(variant === 'bubble' && dialog.frame === true);
	/** the drawn bubble's outline: a box with a tail (speech – an older 'oval' is drawn as a box too) or a wobbly edge (shouting, singing) */
	const shape = $derived(bubble ? (dialog.shape === 'wobbly' ? 'wobbly' : 'box') : null);
	/** an older fixed tail (drawn by CSS below) or a free one (owner 2026-10-18: base on the outline, tip anywhere – tail.ts) */
	const drawnTail = $derived(dialogTail(dialog));
	const tail = $derived(typeof drawnTail === 'string' ? drawnTail : 'none');
	const freeTail = $derived(drawnTail && typeof drawnTail === 'object' ? drawnTail : null);
	/** clip-path ids of the free tail */
	const uid = $props.id();
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
	/** how the box is sized (dialog-fit.ts): auto width hugs its text (CSS max-content, wraps at a max width), auto height
	 *  wraps at its width and grows, fixed keeps both (the fit pass shrinks the text, then it runs past the box) */
	const sizing = $derived(dialogFit(dialog));
	const hasHeight = $derived(sizing === 'fixed' && dialog.rect.height !== undefined);
	const maskColor = $derived(dialog.mask === true ? '#ffffff' : dialog.mask || null);

	let root: HTMLDivElement | null = $state(null);
	let box: HTMLDivElement | null = $state(null);
	let content: HTMLDivElement | null = $state(null);
	/** the box's laid-out size (px): the wobbly outline is drawn for it */
	let boxSize = $state<{ w: number; h: number } | null>(null);
	const wobbly = $derived(shape === 'wobbly' && boxSize ? { d: wobblyPath(boxSize.w, boxSize.h, dialog.id ?? lines.join(' ')) } : null);
	/** the free tail as drawn: measured from the laid-out box and slice (px), null until then – it never moves the layout */
	let tailDrawing = $state<TailDrawing | null>(null);
	function drawTail() {
		tailDrawing = freeTail && root && box ? measureTail(root, box, dialog, freeTail, shape === 'wobbly' ? (wobbly?.d ?? null) : null) : null;
	}

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
		drawTail();
	}

	// the free tail follows its data and the box's place, turn and size (the creator drags it live)
	$effect(() => {
		void [JSON.stringify(freeTail), dialog.rect.x, dialog.rect.y, dialog.anchor, dialog.rotation, dialog.scale, wobbly, box];
		drawTail();
	});

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
	class:fit-width={sizing === 'width'}
	class:framed
	class:bubble
	class:wobbly={shape === 'wobbly'}
	class:stroked
	class:uppercase={dialog.uppercase ?? false}
	data-id={dialog.id}
	data-index={index}
	style:left="{dialog.rect.x}%"
	style:top="{dialog.rect.y}%"
	style:width={sizing === 'width' ? null : `${dialog.rect.width}%`}
	style:max-width={sizing === 'width' && dialog.rect.width ? `${dialog.rect.width}%` : null}
	style:height={hasHeight ? `${dialog.rect.height}%` : null}
	style:translate={anchorTranslate(dialog.anchor)}
	style:--size={dialog.size ?? 4}
	style:--pad={dialog.pad ?? 0.5}
	style:--rotation="{dialog.rotation ?? 0}deg"
	style:--scale={dialog.scale ?? 1}
	style:--dialog-color={dialog.color ?? null}
	style:--dialog-weight={dialog.weight ?? null}
	style:--dialog-stroke={stroked ? dialog.stroke : null}
	style:--dialog-stroke-width={stroked && dialog.strokeWidth !== undefined ? `${dialog.strokeWidth}em` : null}
	style:letter-spacing={dialog.letterSpacing ? `${dialog.letterSpacing}em` : null}
	style:line-height={dialog.lineHeight ?? null}
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
		{#if tailDrawing}
			<!-- a free tail (owner 2026-10-18): the speech wedge or the thought circles, cut at the slice's edge (an off-panel
			     speaker); px of the box's own frame from its border edge (the svg sits at the padding edge) -->
			<svg class="free-tail" aria-hidden="true" data-testid="free-tail" data-kind={tailDrawing.kind}>
				<defs>
					<clipPath id="{uid}-slice"><path d={tailDrawing.slice} /></clipPath>
					<clipPath id="{uid}-out"><path d={tailDrawing.outside} clip-rule="evenodd" /></clipPath>
				</defs>
				<g transform="translate({-tailDrawing.offset.x} {-tailDrawing.offset.y})" clip-path="url(#{uid}-slice)">
					{#if tailDrawing.speech}
						<path class="wedge-fill" d={tailDrawing.speech.fill} />
						<path class="wedge-line" d={tailDrawing.speech.stroke} clip-path="url(#{uid}-out)" stroke-width={tailDrawing.line} />
					{/if}
					{#each tailDrawing.circles as c, i (i)}
						<circle cx={c.cx} cy={c.cy} r={c.r} stroke-width={tailDrawing.line} />
					{/each}
				</g>
			</svg>
		{/if}
		{#if variant === 'lyrics'}
			<!-- a song line (owner 2026-10-17): a ♪ over the top-left corner, a beamed ♫ on the right near the top – drawn,
			     outside the text flow, sized in em of the text -->
			<svg class="note note--single" viewBox="0 0 20 32" aria-hidden="true">
				<ellipse cx="7.4" cy="26.4" rx="6.6" ry="4.8" transform="rotate(-22 7.4 26.4)" />
				<rect x="11.8" y="1.6" width="2.3" height="24.8" />
				<path d="M14.1 1.6 C14.4 6.8 20 8.4 18.9 16.6 C18 12.6 16.4 10.8 14.1 10.2 Z" />
			</svg>
			<svg class="note note--pair" viewBox="0 0 32 32" aria-hidden="true">
				<ellipse cx="6.8" cy="26.5" rx="5.6" ry="4" transform="rotate(-22 6.8 26.5)" />
				<ellipse cx="25.6" cy="23.5" rx="5.6" ry="4" transform="rotate(-22 25.6 23.5)" />
				<rect x="10.5" y="5.6" width="2.1" height="21" />
				<rect x="29.3" y="2.4" width="2.1" height="21" />
				<path d="M10.5 5 L31.4 1.3 L31.4 5.9 L10.5 9.6 Z" />
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
		/* left / top place the anchor point; `translate` (inline, anchor.ts) pulls the drawn box back by its own size, so a
		   box that grows (more lines, another language) grows away from its anchor. Then around the centre: rotation and a
		   uniform scale of the laid-out box (text keeps its wrapping) */
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

	/* auto width: as wide as the longest line (breaks only at a line break), up to the max width – there it wraps; the
	   anchor's translate makes it grow away from its anchor (right anchor → to the left) */
	.fit-width {
		width: max-content;
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

	/* lyrics: the notes sit on the box's border, partly outside (the text keeps its padding); logical sides, so they
	   follow the text direction */
	.dialog--lyrics .box {
		position: relative;
	}

	.note {
		position: absolute;
		height: 2.1em;
		overflow: visible;
		fill: currentColor;
	}

	.note--single {
		width: 1.31em;
		top: -1.3em;
		inset-inline-start: -0.65em;
	}

	.note--pair {
		width: 2.1em;
		top: -0.8em;
		inset-inline-end: -0.95em;
	}

	@media (prefers-reduced-motion: no-preference) {
		.note {
			animation: note-bob 2.8s ease-in-out infinite;
		}

		.note--pair {
			animation-delay: -1.4s;
		}
	}

	@keyframes note-bob {
		50% {
			translate: 0 -0.08em;
		}
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

	/* a free tail: over the box's paper and outline (its paper covers the outline where it joins) */
	.free-tail {
		position: absolute;
		left: 0;
		top: 0;
		width: 100%;
		height: 100%;
		overflow: visible;
		pointer-events: none;
	}

	.free-tail path,
	.free-tail circle {
		fill: var(--paper);
		stroke: none;
	}

	.free-tail .wedge-line,
	.free-tail circle {
		stroke: var(--ink);
		stroke-linejoin: round;
	}

	.free-tail .wedge-line {
		fill: none;
	}

	/* an outline around the letters (e.g. an onomatopoeia: white letters, ink outline) */
	.stroked {
		-webkit-text-stroke: var(--dialog-stroke-width, 0.14em) var(--dialog-stroke);
		paint-order: stroke fill;
	}

</style>
