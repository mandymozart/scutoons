<script lang="ts">
	// Luma – by hand (#121; the studio's ManualGenerate, README "Generators"): free, no API – the prompt built from the
	// shot (read-only in the form above) + Copy prompt, the references to attach on lumalabs.ai (the picked still to
	// download, the figures' character images as links), and the RESULT SLOT: the file made there, dropped here or picked,
	// is saved into the workspace folder (<shot>--g<n>.<ext>) and recorded as luma-manual with the prompt, sha256 and this
	// computer's label. One primary (RULES): Build from the shot until there is a prompt, then Copy prompt.
	import { api } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { ACCEPT } from '$lib/editor/storyboard-generate.ts';
	import { workFile, type Generation } from '$lib/editor/storyboard-generations.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();
	const gen = $derived(sb.gen);
	const still = $derived(gen.form.still === null ? null : (sb.generations.find((x) => x.id === gen.form.still) ?? null));
	const folder = $derived(sb.workspaceState === 'ready');
	let over = $state(false);

	const fail = (e: unknown) => toast(e instanceof Error ? e.message : String(e), { type: 'error' });
	/** the clipboard API, else (no permission – an embedded browser) the old way: the text selected and copied */
	async function copy() {
		try {
			await navigator.clipboard.writeText(gen.form.prompt);
		} catch (e) {
			const area = document.querySelector<HTMLTextAreaElement>('[data-testid="generate-prompt"]');
			area?.select();
			if (!area || !document.execCommand('copy')) throw e;
		}
		toast(m['storyboard.gen_copied'](), { type: 'success' });
	}
	async function download(g: Generation) {
		const blob = g.file && sb.localFiles.has(g.file) ? await sb.readLocal(g.file) : g.work_url ? await workFile(api, g.work_url) : null;
		if (!blob) return;
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = g.file?.split('/').pop() ?? `${g.shot}.png`;
		a.click();
		setTimeout(() => URL.revokeObjectURL(url), 1000);
	}
	async function add(files: FileList | File[] | null | undefined) {
		const file = files?.[0];
		if (!file || gen.adding) return;
		if (!folder) return void toast(m['storyboard.gen_needs_folder'](), { type: 'warning' });
		try {
			const item = await gen.addManual(file);
			toast(m['storyboard.gen_result_added']({ file: item.file ?? '' }), { type: 'success' });
		} catch (e) {
			fail(e);
		}
	}
	const carries = (e: DragEvent) => !!e.dataTransfer?.types.includes('Files');
	function dragover(e: DragEvent) {
		if (!carries(e)) return;
		e.preventDefault();
		e.dataTransfer!.dropEffect = 'copy';
		over = true;
	}
	function drop(e: DragEvent) {
		over = false;
		if (!carries(e)) return;
		e.preventDefault();
		void add(e.dataTransfer?.files);
	}
</script>

{#if gen.form.prompt}
	<div class="buttons">
		<button type="button" onclick={() => copy().catch(fail)} data-testid="manual-copy">{m['storyboard.gen_copy']()}</button>
	</div>
{/if}
{#if still || gen.characters.length}
	<fieldset>
		<legend>{m['storyboard.gen_attach']()}</legend>
		<ul>
			{#if still}<li><button type="button" class="link" onclick={() => download(still).catch(fail)}>{still.file?.split('/').pop() ?? `#${still.id}`}</button> – {m['storyboard.gen_still']()}</li>{/if}
			{#each gen.characters as c (c.key)}<li><a href={c.url} target="_blank" rel="noreferrer" download>{c.label}</a> – {m['storyboard.gen_characters']()}</li>{/each}
		</ul>
	</fieldset>
{/if}
<fieldset>
	<legend>{m['storyboard.gen_result']()}</legend>
	{#if !folder}<p class="hint">{m['storyboard.gen_needs_folder']()}</p>{/if}
	<!-- the media slot (RULES "Media fields are slots": empty = a dashed slot): a click picks the file, a drop adds it -->
	<label class="slot" class:over class:off={!folder || gen.adding} title={m['storyboard.gen_result_slot_hint']()} ondragover={dragover} ondragleave={() => (over = false)} ondrop={drop} data-testid="manual-slot" data-control-exempt="a drop slot">
		{gen.adding ? m['storyboard.gen_result_adding']() : m['storyboard.gen_result_slot']()}
		<input type="file" accept={ACCEPT} hidden disabled={!folder || gen.adding} onchange={(e) => { void add(e.currentTarget.files); e.currentTarget.value = ''; }} data-testid="manual-file" />
	</label>
</fieldset>

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
	.link {
		all: unset;
		cursor: pointer;
		text-decoration: underline;
	}
	.hint {
		margin: 0 0 var(--space-05);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
	}
	.buttons {
		display: flex;
		justify-content: flex-end;
	}
	/* the empty media slot: a dashed frame, its words muted; a file dragged over it: ink */
	.slot {
		display: grid;
		place-items: center;
		box-sizing: border-box;
		width: 100%;
		min-height: 4rem;
		padding: var(--space-05);
		color: var(--text-muted);
		border: var(--border-width-regular) dashed var(--border-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		text-align: center;
		cursor: pointer;
	}
	.slot:hover:not(.off),
	.slot.over {
		border-color: var(--ink);
		color: var(--ink);
	}
	.slot.off {
		cursor: not-allowed;
	}
</style>
