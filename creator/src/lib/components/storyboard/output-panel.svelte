<script lang="ts">
	// The Export step's Output (#126; the studio's OutputPanel – README "Render all handheld shots", "Downloads"): Render
	// every handheld shot (the camera on, an active take) into the workspace folder, one after another; the chapter's videos
	// – the handheld renders and the generated ones – from the ledger, each where it is (this folder, the shared work
	// storage, "on <machine>") with its download.
	import { api } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { workFile } from '$lib/editor/storyboard-generations.ts';
	import { downloadName, handheldShots, outputCounts, outputRows, type OutputRow } from '$lib/editor/storyboard-output.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();

	const rows = $derived(outputRows(sb.generations, sb.order, sb.localFiles));
	const counts = $derived(outputCounts(rows));
	const shots = $derived(handheldShots(sb.entries));
	let busy = $state(false);

	async function renderAll() {
		busy = true;
		let done = 0;
		try {
			for (const { entry, take } of shots) {
				await sb.hh.render(entry, take);
				done++;
			}
			toast(m['storyboard.out_rendered']({ n: done }), { type: 'success' });
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
		} finally {
			busy = false;
		}
	}
	async function download(r: OutputRow) {
		const g = r.g;
		try {
			const blob = r.place.at === 'local' ? await sb.readLocal(g.file!) : r.place.at === 'work' ? await workFile(api, r.place.url) : null;
			if (!blob) return;
			const a = document.createElement('a');
			a.href = URL.createObjectURL(blob);
			a.download = downloadName(sb.toon, sb.chapter, g.file ?? `${g.shot}-${g.id}.webm`);
			a.click();
			setTimeout(() => URL.revokeObjectURL(a.href), 10000);
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
		}
	}
	const where = (r: OutputRow) =>
		r.place.at === 'local' ? m['storyboard.out_here']() : r.place.at === 'work' ? m['storyboard.out_work']() : r.place.at === 'elsewhere' ? m['storyboard.gen_elsewhere']({ machine: r.place.machine ?? m['storyboard.gen_other_machine']() }) : '';
</script>

<div class="panel" data-testid="output-panel">
	<fieldset>
		<legend>{m['storyboard.out_render']()}</legend>
		<div class="rows">
			<p class="sc-caption">{m['storyboard.out_render_hint']({ n: shots.length })}</p>
			<button type="button" onclick={renderAll} disabled={busy || !shots.length || sb.workspaceState !== 'ready'} title={sb.workspaceState === 'ready' ? '' : m['storyboard.hh_render_folder']()} data-testid="out-render-all">
				{sb.hh.rendering ?? m['storyboard.out_render_all']({ n: shots.length })}
			</button>
		</div>
	</fieldset>
	<fieldset>
		<legend>{m['storyboard.out_videos']()}</legend>
		<div class="rows">
			{#if rows.length}
				<p class="sc-caption">{m['storyboard.out_counts']({ total: counts.total, local: counts.local, work: counts.work, elsewhere: counts.elsewhere })}</p>
				<ul class="list">
					{#each rows as r (r.g.id)}
						<li data-testid="out-row-{r.g.id}">
							<span class="name"><span class="cut">{r.g.shot} · {r.kind === 'rendered' ? m['storyboard.tl_rendered']() : m['storyboard.tl_generated']()} · {r.g.file ?? `#${r.g.id}`}</span><span class="sc-caption">{where(r)}{r.g.duration ? ` · ${r.g.duration.toFixed(1)} s` : ''}</span></span>
							<button type="button" class="secondary" disabled={r.place.at !== 'local' && r.place.at !== 'work'} onclick={() => download(r)}>{m['storyboard.out_download']()}</button>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="sc-empty">{m['storyboard.out_none']()}</p>
			{/if}
		</div>
	</fieldset>
</div>

<style>
	.panel {
		display: grid;
		gap: var(--space-1);
		padding: var(--space-1);
	}
	.rows {
		display: grid;
		gap: var(--space-05);
	}
	p {
		margin: 0;
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-05);
	}
	li {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.name {
		flex: 1;
		min-width: 0;
		display: grid;
	}
	.cut {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
