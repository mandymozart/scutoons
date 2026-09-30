<script lang="ts">
	import { onMount } from 'svelte';
	import type { Dialog, DialogFont, DialogVariant } from '@scutoons/content';

	let { dialog }: { dialog: Dialog } = $props();

	/** Smallest scale the auto-fit pass may shrink the font to before it lets the text overflow. */
	const MIN_FIT = 0.8;
	const FIT_STEP = 0.025;

	const variant: DialogVariant = $derived(dialog.variant ?? 'bubble');
	const boxed = $derived(variant === 'caption' || variant === 'narration');
	/** caption / narration draw their paper box + ink border unless `frame: false` */
	const framed = $derived(boxed && dialog.frame !== false);
	const font: DialogFont = $derived(dialog.font ?? (boxed ? 'mono' : 'lettering'));
	const align = $derived(dialog.align ?? (boxed ? 'left' : 'center'));
	const valign = $derived(dialog.valign ?? 'middle');
	const lines = $derived(Array.isArray(dialog.text) ? dialog.text : [dialog.text]);
	const hasHeight = $derived(dialog.rect.height !== undefined);
	const maskColor = $derived(dialog.mask === true ? '#ffffff' : dialog.mask || null);

	let root: HTMLDivElement | null = $state(null);
	let box: HTMLDivElement | null = $state(null);
	let content: HTMLDivElement | null = $state(null);

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
	}

	// re-fit whenever the text or geometry changes (e.g. locale switch)
	$effect(() => {
		void [lines, dialog.size, dialog.rect.width, dialog.rect.height, font, variant, framed];
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
	class:uppercase={dialog.uppercase ?? variant === 'sfx'}
	data-id={dialog.id}
	style:left="{dialog.rect.x}%"
	style:top="{dialog.rect.y}%"
	style:width="{dialog.rect.width}%"
	style:height={hasHeight ? `${dialog.rect.height}%` : null}
	style:--size={dialog.size ?? 4}
	style:--pad={dialog.pad ?? 0.5}
	style:--rotation="{dialog.rotation ?? 0}deg"
	style:--dialog-color={dialog.color ?? null}
	style:--dialog-weight={dialog.weight ?? null}
>
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
		<div bind:this={content} class="content" style:text-align={align}>
			{#each lines as line, index (index)}
				<span class="line">{line}</span>
			{/each}
		</div>
	</div>
</div>

<style>
	.dialog {
		--ink: #000000;
		--paper: #ffffff;
		--fit: 1;
		position: absolute;
		box-sizing: border-box;
		isolation: isolate;
		transform: rotate(var(--rotation));
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

	/* sfx: outlined lettering like the chapter-1 onomatopoeia (white fill, ink outline) */
	.dialog--sfx {
		color: var(--dialog-color, var(--paper));
		-webkit-text-stroke: 0.14em var(--ink);
		paint-order: stroke fill;
		line-height: 1;
	}
</style>
