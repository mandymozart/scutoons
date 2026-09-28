<script lang="ts">
	import { onMount } from 'svelte';
	import type { Dialog, DialogFont, DialogVariant } from '$lib/models/content.ts';

	let { dialog }: { dialog: Dialog } = $props();

	/** Smallest scale the auto-fit pass may shrink the font to before it lets the text overflow. */
	const MIN_FIT = 0.8;
	const FIT_STEP = 0.025;

	const variant: DialogVariant = $derived(dialog.variant ?? 'bubble');
	const boxed = $derived(variant === 'caption' || variant === 'narration');
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

	/** Shrink the font (down to MIN_FIT) until the text fits a box with a fixed height. */
	function fit() {
		if (!root) return;
		let scale = 1;
		root.style.setProperty('--fit', '1');
		if (!hasHeight) return;
		while (scale > MIN_FIT && overflows()) {
			scale = Math.max(MIN_FIT, scale - FIT_STEP);
			root.style.setProperty('--fit', String(scale));
		}
	}

	// re-fit whenever the text or geometry changes (e.g. locale switch)
	$effect(() => {
		void [lines, dialog.size, dialog.rect.width, dialog.rect.height, font, variant];
		fit();
	});

	onMount(() => {
		let frame = 0;
		const schedule = () => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(fit);
		};
		const observer = new ResizeObserver(schedule);
		if (box) observer.observe(box);
		document.fonts?.ready.then(schedule);
		return () => {
			cancelAnimationFrame(frame);
			observer.disconnect();
		};
	});
</script>

<div
	bind:this={root}
	class="dialog dialog--{variant} font-{font}"
	class:has-height={hasHeight}
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
		font-family: 'Bangers', 'IBM Plex Mono', monospace;
		line-height: 1.1;
		letter-spacing: 0.03em;
	}

	.font-mono {
		font-family: 'IBM Plex Mono', monospace;
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

	/* caption / narration: on-brand panel caption (paper box, 2px ink border, square) */
	.dialog--caption .box,
	.dialog--narration .box {
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
