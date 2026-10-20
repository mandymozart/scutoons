<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { onMount, untrack } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { SOURCE_MARK, textLanguage } from '$lib/editor/text-language.svelte.ts';

	/**
	 * A text box's words edited in place on the slice canvas (owner 2026-10-17: "edit text boxes in place – currently that
	 * is only possible with the side menu"): a double click on the box opens it, in the canvas's language, set like the
	 * box (font, size, alignment, case, spacing) so the lines wrap as in the reader. Every keystroke writes the document
	 * (the inspector's text field shows the same value – one state); Escape puts the text from before back, a click
	 * outside or Ctrl / Cmd + Enter closes it. Enter is a line break, like in the side field. Paragraph lists (`string[]`)
	 * are edited with a blank line between entries and stay lists (as in LocaleText).
	 *
	 * `bar` (the phone, agents/research/TEXT-ACROSS-SLICES.md §4.1 E3 – a tap on the text opens it): one bar above the
	 * keyboard – ✕ (the old words back; there only after a change), the language chip (the header's text language switch –
	 * the whole view's language; ★ the source, a dot for a missing translation; codes only, names as tooltip) and ✓ done. A
	 * translation shows the source as its placeholder and offers "copy the original".
	 */
	let {
		doc,
		path,
		locale,
		dialog,
		boxed,
		bar = false,
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
		/** the phone's bar above the keyboard */
		bar?: boolean;
		onclose: () => void;
	} = $props();

	const textPath = $derived([...path, 'text']);
	const key = $derived(`text:${locale}:${textPath.join('.')}`);
	/** the text of each language when the editor first showed it (Escape / ✕ put it back) */
	const befores = new Map<string, unknown>();
	const before = $derived.by(() => {
		if (!befores.has(locale)) befores.set(locale, untrack(() => doc.text([...path, 'text'], locale)));
		return befores.get(locale);
	});
	const asList = untrack(() => Array.isArray(doc.text([...path, 'text'], locale)));
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
			revert();
			onclose();
		} else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			onclose();
		}
	}

	function revert() {
		doc.setText(textPath, locale, (before ?? '') as string | string[], key);
	}
	const flat = (v: unknown) => (Array.isArray(v) ? v.join('\n\n') : typeof v === 'string' ? v : '');
	/** ✕ only after a change (absent, not disabled) */
	const changed = $derived(flat(before) !== value);
	const source = $derived(doc.textSource);
	const sourceText = $derived(locale === source ? '' : flat(doc.text(textPath, source)));
	const languages = $derived(doc.textLanguages);
	const missing = (l: string) => !flat(doc.text(textPath, l)).trim();
	let listing = $state(false);
	/** the bar sits on the keyboard: the visual viewport's bottom (rem – DESIGN.md) */
	let lift = $state(0);

	onMount(() => {
		area?.focus();
		area?.select();
		if (!bar || !window.visualViewport) return;
		const vv = window.visualViewport;
		const place = () => {
			const r = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
			lift = Math.max(0, innerHeight - vv.height - vv.offsetTop) / r;
		};
		place();
		vv.addEventListener('resize', place);
		vv.addEventListener('scroll', place);
		return () => {
			vv.removeEventListener('resize', place);
			vv.removeEventListener('scroll', place);
		};
	});
	/** the bar lives on the body: a turned / scaled box would make `fixed` relative to itself */
	function toBody(node: HTMLElement) {
		document.body.appendChild(node);
		return () => node.remove();
	}
	/** a bar button keeps the focus in the text (a blur would close the editor first) */
	const keep = (e: PointerEvent) => e.preventDefault();
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
	placeholder={sourceText || null}
	oninput={(e) => write(e.currentTarget.value)}
	onblur={onclose}
	{onkeydown}
	onpointerdown={(e) => e.stopPropagation()}
	ondblclick={(e) => e.stopPropagation()}
	data-testid="canvas-text-edit"
></textarea>
{#if bar}
	<!-- the phone's bar above the keyboard (E3) -->
	<div class="edit-bar" style:bottom="{lift}rem" {@attach toBody} data-testid="text-edit-bar">
		<button type="button" class="sq" class:absent={!changed} aria-hidden={!changed} tabindex={changed ? 0 : -1} title={m['creator.text_revert']()} aria-label={m['creator.text_revert']()} onpointerdown={keep} onclick={() => { revert(); onclose(); }} data-testid="text-edit-revert"><Icon name="x" /></button>
		<span class="grow"></span>
		{#if sourceText}
			<button type="button" class="sq" title={m['creator.text_copy_original']()} aria-label={m['creator.text_copy_original']()} onpointerdown={keep} onclick={() => write(sourceText)} data-testid="text-edit-copy-source"><Icon name="clipboard" /></button>
		{/if}
		<span class="lang">
			<button type="button" class="chip" class:translated={locale !== source} aria-haspopup="listbox" aria-expanded={listing} title={m['creator.text_language']()} onpointerdown={keep} onclick={() => (listing = !listing)} data-testid="text-edit-language">
				<Icon name="globe" size="1rem" />{locale.toUpperCase()}{#if locale === source}&nbsp;{SOURCE_MARK}{/if}
			</button>
			{#if listing}
				<span class="langs" role="listbox" aria-label={m['creator.text_language']()}>
					{#each languages as l (l)}
						<button
							type="button"
							role="option"
							aria-selected={l === locale}
							title={l === source ? m['creator.text_language_source']() : missing(l) ? m['creator.text_language_missing']() : m['creator.text_language_translated']()}
							onpointerdown={keep}
							onclick={() => {
								listing = false;
								textLanguage.set(l);
							}}
							data-testid="text-edit-language-{l}">{l.toUpperCase()}{#if l === source}&nbsp;{SOURCE_MARK}{:else if missing(l)}<span class="missing"></span>{/if}</button
						>
					{/each}
				</span>
			{/if}
		</span>
		<span class="grow"></span>
		<button type="button" class="sq done" title={m['creator.text_done']()} aria-label={m['creator.text_done']()} onpointerdown={keep} onclick={onclose} data-testid="text-edit-done"><Icon name="check" /></button>
	</div>
{/if}

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
	/* the phone's bar above the keyboard: squares as the bottom row's; the language chip is the header's switch */
	.edit-bar {
		position: fixed;
		left: 0;
		right: 0;
		z-index: 45;
		display: flex;
		align-items: center;
		background: var(--paper);
		border-top: var(--border-width-regular) solid var(--ink);
		font-size: var(--type-body-size, 1rem);
		text-transform: none;
	}
	.edit-bar .grow {
		flex: 1;
	}
	.edit-bar .sq {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--ink);
	}
	.edit-bar .sq.absent {
		visibility: hidden;
	}
	.edit-bar .done {
		background: var(--ink);
		color: var(--paper);
	}
	.lang {
		position: relative;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
		min-height: calc(var(--control-height) - 2 * var(--space-025));
		padding: 0 var(--space-05);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
	}
	/* a translation is shown: the chip inverted (like the header's switch) */
	.chip.translated {
		background: var(--ink);
		color: var(--paper);
	}
	.langs {
		position: absolute;
		bottom: calc(100% + var(--space-025));
		left: 50%;
		translate: -50% 0;
		display: flex;
		flex-direction: column;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
	}
	.langs button {
		position: relative;
		min-width: 4rem;
		min-height: var(--control-height);
		padding: 0 var(--space-1);
		border: 0;
		background: transparent;
		color: var(--ink);
		text-align: left;
	}
	.langs button[aria-selected='true'] {
		background: var(--ink);
		color: var(--paper);
	}
	.missing {
		display: inline-block;
		width: 0.5rem;
		height: 0.5rem;
		margin-left: var(--space-025);
		border-radius: 50%;
		background: var(--state-error);
	}
	.canvas-text-edit:focus-visible {
		outline: var(--border-width-regular) solid var(--select-live-border);
	}
</style>
