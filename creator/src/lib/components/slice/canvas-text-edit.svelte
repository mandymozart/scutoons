<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { onMount, untrack } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A text box's words edited in place on the slice canvas (owner 2026-10-17: "edit text boxes in place – currently that
	 * is only possible with the side menu"): a double click on the box opens it, in the canvas's language, set like the
	 * box (font, size, alignment, case, spacing) so the lines wrap as in the reader. Every keystroke writes the document
	 * (the inspector's text field shows the same value – one state); Escape puts the text from before back, a click
	 * outside or Ctrl / Cmd + Enter closes it. Enter is a line break, like in the side field. Paragraph lists (`string[]`)
	 * are edited with a blank line between entries and stay lists (as in LocaleText).
	 */
	let {
		doc,
		path,
		locale,
		dialog,
		boxed,
		onclose
	}: {
		doc: ContentDoc;
		/** the text box's path (its `text` is under it) */
		path: Path;
		locale: string;
		/** the box as shown (its look) */
		dialog: any;
		/** caption / narration / lyrics (their defaults differ from a bubble's) */
		boxed: boolean;
		onclose: () => void;
	} = $props();

	const textPath = $derived([...path, 'text']);
	const key = $derived(`text:${locale}:${textPath.join('.')}`);
	/** the text when the editor opened (Escape puts it back) */
	const before = untrack(() => doc.text([...path, 'text'], locale));
	const asList = Array.isArray(before);
	let area: HTMLTextAreaElement | undefined = $state();

	const value = $derived.by(() => {
		const v = doc.text(textPath, locale);
		return Array.isArray(v) ? v.join('\n\n') : typeof v === 'string' ? v : '';
	});
	const font = $derived(dialog?.font ?? (boxed ? 'mono' : 'lettering'));

	function write(input: string) {
		const next = asList && input ? input.split(/\n{2,}/).map((p) => p.replace(/^\n+|\n+$/g, '')) : input;
		doc.setText(textPath, locale, next, key);
	}

	function onkeydown(e: KeyboardEvent) {
		e.stopPropagation();
		if (e.key === 'Escape') {
			e.preventDefault();
			doc.setText(textPath, locale, before ?? '', key);
			onclose();
		} else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			onclose();
		}
	}

	onMount(() => {
		area?.focus();
		area?.select();
	});
</script>

<!-- svelte-ignore a11y_autofocus -->
<textarea
	bind:this={area}
	class="canvas-text-edit font-{font}"
	class:uppercase={dialog?.uppercase ?? false}
	class:framed={boxed ? dialog?.frame !== false : dialog?.frame === true}
	style:--size={dialog?.size ?? 4}
	style:text-align={dialog?.align ?? (boxed ? 'left' : 'center')}
	style:font-weight={dialog?.weight ?? 400}
	style:letter-spacing={dialog?.letterSpacing !== undefined ? `${dialog.letterSpacing}em` : null}
	style:line-height={dialog?.lineHeight ?? null}
	aria-label="{m['creator.field_text']()} · {locale}"
	data-focus-own
	spellcheck="true"
	{value}
	oninput={(e) => write(e.currentTarget.value)}
	onblur={onclose}
	{onkeydown}
	onpointerdown={(e) => e.stopPropagation()}
	ondblclick={(e) => e.stopPropagation()}
	data-testid="canvas-text-edit"
></textarea>

<style>
	/* over the box, set like the reader's text (shared/reader dialog.svelte: size in cqw of the slice – here --cqw, 1 % of
	   the canvas: no container around the boxes, slice-canvas.svelte); paper behind it while editing, the live frame
	   around it */
	.canvas-text-edit {
		position: absolute;
		inset: 0 0 auto;
		box-sizing: border-box;
		width: 100%;
		height: auto;
		min-height: 100%;
		field-sizing: content;
		margin: 0;
		padding: 0;
		border: 0;
		outline: var(--border-width-regular) solid var(--select-live-border);
		background: var(--paper);
		color: var(--ink);
		font-size: calc(var(--size) * var(--cqw, 1cqw));
		line-height: 1.15;
		white-space: pre-wrap;
		resize: none;
		cursor: text;
	}
	.font-lettering {
		font-family: var(--font-lettering);
		line-height: 1.1;
		letter-spacing: 0.03em;
	}
	.font-mono {
		font-family: var(--font-mono);
		line-height: 1.35;
	}
	.uppercase {
		text-transform: uppercase;
	}
	/* a drawn box / bubble insets its text like the reader (dialog.svelte) */
	.framed {
		padding: 0.8em 1em;
	}
	.canvas-text-edit:focus-visible {
		outline: var(--border-width-regular) solid var(--select-live-border);
	}
</style>
