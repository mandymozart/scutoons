<script>
	// Luma – by hand in the Generate panel (generators/luma-manual.mjs; owner 2026-10-19: "the luma tab is weird. this
	// should be somehow now be covered by the generate right?"): what the Luma panel offered, in Generate's place – the
	// prompt built from the shot (GeneratePanel's Prompt row, read-only; while a board card fills the form: the card's,
	// editable), the render without texts (Luma's image reference), the references to attach – and the RESULT SLOT
	// (agents/research/VARIANTS-CANVAS.md §3.7 "Manual", issue #62 stage 3): the file made on lumalabs.ai, dropped here or
	// picked, is recorded as THIS generator's candidate – model 'manual', the form's prompt and reference (the Still), the
	// parent (the card it improves on) – not as an upload; a drop on the board stays a source (board-sources.svelte.js).
	// One primary (RULES): "Build from the shot" until there is a prompt, then Copy prompt. README "Generators".
	import { fail, notify, studio } from '../studio.svelte.js';
	import { framePng } from '../frames.js';
	import { gen, manualPrompt } from '../generate.svelte.js';
	import { DROP_TYPES, carriesFiles } from '../board-sources.svelte.js';
	import { addResult, resultParent } from '../improve.svelte.js';
	import { shortId } from '../board-layout.js';

	/** built: the prompt is built for the current shot; ready: the form holds its inputs (the slot waits for them) */
	let { built, ready = true } = $props();
	const text = $derived(manualPrompt());
	const parent = $derived(resultParent());
	let picker = $state();
	let over = $state(false);
	let adding = $state(false);

	async function copy() {
		await navigator.clipboard.writeText(text);
		notify('Prompt copied – paste it into Luma.', 'success');
	}
	async function downloadRender() {
		const a = document.createElement('a');
		a.href = await framePng({ clean: true });
		a.download = `${studio.sel.id}-structure.png`;
		a.click();
	}
	async function add(files) {
		const file = files?.[0];
		if (!file || adding) return;
		if (!DROP_TYPES.includes(file.type)) return notify(`${file.name}: not added – png, jpeg, webp, mp4 or webm only.`, 'error');
		adding = true;
		try { await addResult([file]); } catch (e) { fail(e); } finally { adding = false; }
	}
	function dragover(e) {
		if (!carriesFiles(e)) return;
		e.preventDefault();
		e.dataTransfer.dropEffect = 'copy';
		over = true;
	}
	function drop(e) {
		over = false;
		if (!carriesFiles(e)) return;
		e.preventDefault();
		add([...e.dataTransfer.files]);
	}
</script>

<!-- right under the prompt (Copy stays in sight above the long list), the primary last -->
<div class="buttons">
	<button type="button" class="secondary" onclick={() => downloadRender().catch(fail)} title="The render as it is now, without texts – Luma's image reference">Render (no texts)</button>
	{#if text}<button type="button" onclick={() => copy().catch(fail)}>Copy prompt</button>{/if}
</div>
{#if built && gen.built?.refs?.length}
	<fieldset>
		<legend>Attach in Luma</legend>
		<ul>
			{#each gen.built.refs as r, i (i)}
				<li>{#if r.url}<a href={r.url} download={r.name ?? ''}>{r.name}</a>{:else}<span class="muted">no image yet</span>{/if} – {r.role}</li>
			{/each}
		</ul>
	</fieldset>
{/if}
{#if ready}<fieldset>
	<legend>Result</legend>
	<!-- the media slot (RULES "Media fields are slots": empty = a dashed slot): a click picks the file, a drop adds it -->
	<button type="button" class="slot" class:over disabled={adding} title="The picture or clip you made on lumalabs.ai – recorded as Luma – by hand's result with this prompt{gen.form.still ? ' and the reference' : ''}{parent ? `, under ${shortId(parent)} on the board` : ''} (png, jpeg, webp, mp4, webm)" onclick={() => picker.click()} ondragover={dragover} ondragleave={() => (over = false)} ondrop={drop} data-testid="manual-slot">
		{adding ? 'Adding …' : `Drop Luma's result here – or pick it${parent ? ` (under ${shortId(parent)})` : ''}`}
	</button>
	<input bind:this={picker} type="file" accept={DROP_TYPES.join(',')} hidden onchange={(e) => { add([...e.currentTarget.files]); e.currentTarget.value = ''; }} data-testid="manual-file" />
</fieldset>{/if}

<style>
	fieldset {
		margin: 0;
	}
	ul {
		margin: 0;
		padding-left: var(--space-1);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		overflow-wrap: anywhere;
	}
	.muted {
		color: var(--text-muted);
	}
	/* the primary last (RULES); a row too narrow for both wraps, each keeps its words on one line */
	.buttons {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--space-05);
	}
	.buttons button {
		white-space: nowrap;
	}
	/* the empty media slot: a dashed frame, its words muted; a file dragged over it: ink */
	.slot {
		width: 100%;
		min-height: 4rem;
		padding: var(--space-05);
		background: transparent;
		color: var(--text-muted);
		border: var(--border-width-regular) dashed var(--border-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		white-space: normal;
		text-align: center;
	}
	.slot:hover:not(:disabled),
	.slot.over {
		border-color: var(--ink);
		color: var(--ink);
	}
</style>
