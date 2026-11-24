<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { Button, IconButton, Pill } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import type { ProseHandle, ProseSelection } from '$lib/editor/prose-cm.ts';
	import { selectionState, toggleDirection, toggleLinePrefix, toggleWrap, toLink, toSpeaker, type WritingEdit } from '$lib/editor/writing.ts';

	/**
	 * The text of a writing page (lap 2 C14): it stands on the stage, Markdown shown as you type; the tools come to the
	 * selection as one pill (bold, italic, heading, quote, link · speaker, direction) – nothing sits above the text. The
	 * text reaches the page after a short pause (`onchange`), before a save (`flush`) and when it loses the focus.
	 */
	let {
		text,
		readonly = false,
		label,
		lang,
		placeholder = '',
		onchange,
		onlockedkey,
		oncaret,
		testid = 'writing-editor'
	}: {
		text: string;
		readonly?: boolean;
		label: string;
		lang: string;
		placeholder?: string;
		onchange: (text: string) => void;
		/** a key while read-only: true when the page took it */
		onlockedkey?: (event: KeyboardEvent) => boolean;
		/** the caret's 1-based line (the Outline marks its section) */
		oncaret?: (line: number) => void;
		testid?: string;
	} = $props();

	const SYNC_MS = 300;
	let host = $state<HTMLDivElement>();
	let handle: ProseHandle | null = null;
	let pending = false;
	let timer: ReturnType<typeof setTimeout> | null = null;
	let selection = $state<ProseSelection | null>(null);
	let marks = $state<ReturnType<typeof selectionState> | null>(null);

	export function flush() {
		if (timer) clearTimeout(timer);
		timer = null;
		if (!pending || !handle) return;
		pending = false;
		onchange(handle.text());
	}
	export function jump(line: number) {
		flush();
		handle?.jump(line);
	}
	export function focus() {
		handle?.focus();
	}

	onMount(() => {
		let gone = false;
		void import('$lib/editor/prose-cm.ts').then(({ createProse }) => {
			if (gone || !host) return;
			handle = createProse(host, {
				text: untrack(() => text),
				readOnly: untrack(() => readonly),
				label: untrack(() => label),
				lang: untrack(() => lang),
				placeholder: untrack(() => placeholder),
				onChange: () => {
					pending = true;
					if (timer) clearTimeout(timer);
					timer = setTimeout(flush, SYNC_MS);
				},
				onSelection: (s) => {
					selection = s;
					marks = s.box && handle ? selectionState(handle.text(), s.from, s.to) : null;
					if (handle) oncaret?.(handle.caretLine());
				},
				onLockedKey: (event) => onlockedkey?.(event) ?? false,
				onBlur: flush
			});
		});
		return () => {
			gone = true;
			flush();
			handle?.destroy();
			handle = null;
		};
	});

	// the page's text into the editor (load, restore, a language switch) – never over an edit not handed on yet
	$effect(() => {
		const next = text;
		untrack(() => {
			if (handle && !pending) handle.setText(next);
		});
	});
	$effect(() => {
		const r = readonly;
		untrack(() => handle?.setReadOnly(r));
	});

	function run(make: (text: string, from: number, to: number) => WritingEdit) {
		handle?.apply(make);
	}
	/** the pill keeps the selection: a press on it must not take the focus from the text */
	const keep = (event: PointerEvent) => event.preventDefault();
	const tools = $derived([
		{ id: 'strong', glyph: 'B', label: m['creator.writing_bold'](), on: marks?.strong, run: () => run((t, a, b) => toggleWrap(t, a, b, '**')) },
		{ id: 'em', glyph: 'I', label: m['creator.writing_italic'](), on: marks?.em, run: () => run((t, a, b) => toggleWrap(t, a, b, '*')) },
		{ id: 'heading', glyph: 'H', label: m['creator.writing_heading'](), on: marks?.heading, run: () => run((t, a, b) => toggleLinePrefix(t, a, b, '## ')) },
		{ id: 'quote', glyph: '“', label: m['creator.writing_quote'](), on: marks?.quote, run: () => run((t, a, b) => toggleLinePrefix(t, a, b, '> ')) }
	]);
</script>

<div class="prose" data-testid={testid}>
	<div class="host" bind:this={host}></div>
	{#if selection?.box && !readonly}
		<div class="pill-at" style:left="{selection.box.left}px" style:top="{selection.box.bottom}px" onpointerdown={keep} role="presentation">
			<Pill label={m['creator.writing_tools']()} data-testid="writing-pill">
				{#each tools as tool (tool.id)}
					<IconButton label={tool.label} pressed={tool.on} onclick={tool.run} data-testid="writing-{tool.id}"><span class="glyph {tool.id}">{tool.glyph}</span></IconButton>
				{/each}
				<IconButton label={m['creator.writing_link']()} icon="link" onclick={() => run(toLink)} data-testid="writing-link" />
				<span class="seam" aria-hidden="true"></span>
				<Button variant="quiet" pressed={marks?.speaker} title={m['creator.writing_speaker_about']()} onclick={() => run((t, a, b) => toSpeaker(t, a, b, lang))} data-testid="writing-speaker">{m['creator.writing_speaker']()}</Button>
				<Button variant="quiet" pressed={marks?.direction} title={m['creator.writing_direction_about']()} onclick={() => run(toggleDirection)} data-testid="writing-direction">{m['creator.writing_direction']()}</Button>
			</Pill>
		</div>
	{/if}
</div>

<style>
	.prose {
		position: relative;
	}
	.host {
		min-height: 1lh;
	}
	/* under the selection, a little apart; a popover's place (RULES H15: no blur) */
	.pill-at {
		position: fixed;
		z-index: 40;
		margin-top: 0.5rem;
		font-size: var(--editor-font);
		line-height: normal;
		font-style: normal;
		letter-spacing: normal;
		text-transform: none;
	}
	.glyph {
		font-size: var(--editor-font);
		line-height: 1;
	}
	.glyph.em {
		font-style: italic;
	}
	.seam {
		align-self: stretch;
		width: var(--editor-edge);
		margin: var(--kit-inset) 0;
		background: var(--tool-line);
	}
</style>
