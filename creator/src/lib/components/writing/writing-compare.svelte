<script lang="ts">
	import { untrack } from 'svelte';
	import { joinParagraphs, paragraphs } from '$lib/editor/writing.ts';
	import ProseEditor from '$lib/components/writing/prose-editor.svelte';
	import ReadParagraph from '$lib/components/writing/read-paragraph.svelte';
	import WritingHead from '$lib/components/writing/writing-head.svelte';

	/**
	 * Compare (lap 2 C14b): the source left, read-only and quieter, the edit language right – two reading columns of the
	 * same width, paragraph beside paragraph. A paragraph not translated yet is an empty line to fill, never a warning
	 * (RULES H17). The edit language's text reaches the page joined again (`onchange`).
	 */
	let {
		source,
		target,
		readonly = false,
		label,
		onchange,
		onlockedkey,
		onprogress
	}: {
		source: { text: string; locale: string; title: string; sub: string; lang: string };
		target: { text: string; locale: string; title: string; sub: string; lang: string };
		readonly?: boolean;
		label: string;
		onchange: (text: string) => void;
		onlockedkey?: (event: KeyboardEvent) => boolean;
		/** how many of the source's paragraphs have a text beside them */
		onprogress?: (done: number, total: number) => void;
	} = $props();

	const sourceRows = $derived(paragraphs(source.text));
	const padded = (list: string[], length: number) => [...list, ...Array<string>(Math.max(0, length - list.length)).fill('')];
	// svelte-ignore state_referenced_locally
	let rows = $state<string[]>(padded(paragraphs(target.text), paragraphs(source.text).length));
	let joined = joinParagraphs(untrack(() => rows));

	// the edit language's text from outside (a restore, the page's undo): the rows again
	$effect(() => {
		const text = target.text;
		const length = sourceRows.length;
		untrack(() => {
			if (text === joined) {
				if (rows.length < length) rows = padded(rows, length);
				return;
			}
			joined = text;
			rows = padded(paragraphs(text), length);
		});
	});
	$effect(() => {
		const done = rows.slice(0, sourceRows.length).filter((r) => r.trim()).length;
		onprogress?.(done, sourceRows.length);
	});

	function write(i: number, text: string) {
		const next = [...rows];
		next[i] = text;
		rows = next;
		joined = joinParagraphs(next);
		onchange(joined);
	}
</script>

<div class="compare" data-testid="writing-compare-view">
	<div class="head source"><WritingHead title={source.title} sub={source.sub} lang={source.lang} testid="compare-source-title" /></div>
	<div class="head"><WritingHead title={target.title} sub={target.sub} lang={target.lang} testid="compare-target-title" /></div>
	{#each rows as row, i (i)}
		<div class="source" data-testid="compare-source">{#if sourceRows[i] !== undefined}<ReadParagraph text={sourceRows[i]} lang={source.locale} />{/if}</div>
		<div class="target" class:empty={!row.trim()} data-testid="compare-target">
			<ProseEditor text={row} {readonly} {label} lang={target.locale} {onlockedkey} onchange={(t) => write(i, t)} testid="compare-editor" />
		</div>
	{/each}
</div>

<style>
	/* two columns of the same width, a hairline between them, each paragraph's row as high as the longer side – wider
	   than the stage's page column, reaching out to both sides */
	.compare {
		position: relative;
		left: 50%;
		translate: -50%;
		width: min(78rem, 100vw - 4rem);
		box-sizing: border-box;
		--wr-size: 1.08rem;
		--wr-title: 1.65em;
		display: grid;
		grid-template-columns: minmax(0, 33em) minmax(0, 33em);
		column-gap: 3.5rem;
		row-gap: 1.1em;
		justify-content: center;
		align-items: start;
		padding: 2.5rem 1.5rem 6rem;
		font-size: var(--wr-size);
		line-height: 1.55;
		background: linear-gradient(var(--tool-line), var(--tool-line)) center / var(--editor-edge) 100% no-repeat;
	}
	.head {
		align-self: end;
	}
	.source {
		opacity: 0.62;
	}
	.head.source {
		opacity: 1;
	}
	/* a paragraph not written yet: an empty line to fill */
	.target.empty :global(.cm-content) {
		border-bottom: var(--editor-edge) dashed var(--tool-line);
	}
</style>
