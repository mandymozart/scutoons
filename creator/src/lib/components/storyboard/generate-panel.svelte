<script lang="ts">
	// The Generate panel (#123, #121): on top the form that starts a generation (generate-form – a generator per shot, the
	// estimate in euro, the confirmation with the cost; Luma – by hand: generate-manual), the shot's running jobs
	// (generate-jobs), then the shot's generations – the approved one first, then the newest – each with its file
	// (generation-slot), who made it with what, its mark (for review / rejected), Approve and Share (the local file into
	// the shared work storage, so the others see it – never the readers' CDN). A finished provider job not in a folder yet:
	// Save to folder (or a line to choose one). The variants board is #125.
	import { Segmented } from '@scutoons/ui';
	import { Button } from '@scutoons/ui/editor';
	import type { GenerationMark } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { approvePatch, placeOf, shotGenerations, type Generation } from '$lib/editor/storyboard-generations.ts';
	import { isOpen } from '$lib/editor/storyboard-generate.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import GenerationSlot from '$lib/components/storyboard/generation-slot.svelte';
	import GenerateForm from '$lib/components/storyboard/generate-form.svelte';
	import GenerateJobs from '$lib/components/storyboard/generate-jobs.svelte';
	import ShotAssetSlot from '$lib/components/storyboard/shot-asset-slot.svelte';

	let { sb }: { sb: StoryboardPage } = $props();

	const list = $derived(sb.current ? shotGenerations(sb.generations, sb.current).filter((g) => !isOpen(g)) : []);
	let busy = $state<number | null>(null);
	const MARKS: { value: GenerationMark; label: string }[] = $derived([
		{ value: 'review', label: m['storyboard.mark_review']() },
		{ value: 'rejected', label: m['storyboard.mark_rejected']() }
	]);

	async function run(g: Generation, job: () => Promise<void>, done?: string) {
		busy = g.id;
		try {
			await job();
			if (done) toast(done, { type: 'success', kind: 'generation' });
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error', kind: 'generation' });
		} finally {
			busy = null;
		}
	}
	const made = (g: Generation) => [g.generator, g.model, g.created_at ? new Date(g.created_at).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' }) : null, g.author].filter(Boolean).join(' · ');
	const placeWord = (g: Generation) => {
		const p = placeOf(g, sb.localFiles);
		return p.at === 'local' ? m['storyboard.gen_local']() : p.at === 'work' ? m['storyboard.gen_work']() : '';
	};
</script>

<div class="panel" data-testid="generate-panel">
	{#if sb.sel}
		<span class="sc-label">{m['storyboard.slot_title']({ id: sb.sel.id })}</span>
		<ShotAssetSlot {sb} />
		<span class="sc-label by-hand">{m['storyboard.slot_by_hand']()}</span>
	{/if}
	<GenerateForm {sb} />
	<GenerateJobs {sb} />
	{#if sb.generationsError}
		<p class="sc-error">{m['storyboard.gen_error']({ error: sb.generationsError })}</p>
	{:else if list.length}
		<p class="muted">{m['storyboard.gen_count']({ count: list.length })}</p>
	{/if}
	{#each list as g (g.id)}
		<article class:approved={g.approved} data-testid="generation-{g.id}">
			<GenerationSlot {sb} {g} />
			<p class="made">{made(g)}{#if placeWord(g)} · {placeWord(g)}{/if}</p>
			{#if g.status === 'failed'}<p class="sc-error">{m['storyboard.gen_failed']({ error: g.error ?? '' })}</p>{/if}
			{#if g.note}<p class="note">{g.note}</p>{/if}
			{#if placeOf(g, sb.localFiles).at === 'result'}
				{#if sb.workspaceState === 'ready'}
					<div class="actions">
						<span class="grow"></span>
						<Button icon="download" title={m['storyboard.gen_save_hint']()} disabled={sb.gen.saving === g.id} onclick={() => run(g, async () => void toast(m['storyboard.gen_saved']({ file: (await sb.gen.save(g)).file ?? '' }), { type: 'success', kind: 'generation' }))} data-testid="save-{g.id}">{m['storyboard.gen_save']()}</Button>
					</div>
				{:else}
					<p class="muted">{m['storyboard.gen_result_pick_folder']()}</p>
				{/if}
			{/if}
			<div class="actions">
				<Segmented label={m['storyboard.gen_mark']()} options={MARKS} value={g.mark} onchange={(mark) => run(g, () => sb.review(g, { mark }))} onclear={() => run(g, () => sb.review(g, { mark: null }))} />
				<span class="grow"></span>
				{#if placeOf(g, sb.localFiles).at === 'local' && !g.work_path}
					<Button icon="upload-cloud" title={m['storyboard.gen_share_hint']()} disabled={busy === g.id} onclick={() => run(g, () => sb.share(g), m['storyboard.gen_shared']())} data-testid="share-{g.id}">{m['storyboard.gen_share']()}</Button>
				{/if}
				<Segmented label={m['storyboard.gen_approve']()} options={[{ value: 'approved', icon: 'check', label: m['storyboard.gen_approve'](), title: m['storyboard.gen_approve_hint'](), disabled: busy === g.id || g.status !== 'succeeded' }]} value={g.approved ? 'approved' : null} onchange={() => run(g, () => sb.review(g, approvePatch(g)))} onclear={() => run(g, () => sb.review(g, approvePatch(g)))} testid="approve-{g.id}" />
			</div>
		</article>
	{/each}
</div>

<style>
	.panel {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--space-05);
		align-content: start;
		padding: var(--editor-pad);
		box-sizing: border-box;
	}
	.by-hand {
		margin-top: var(--space-05);
	}
	p {
		margin: 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.muted,
	.made {
		color: var(--tool-muted);
	}
	article {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--space-05);
		padding-inline: var(--space-05);
	}
	article.approved {
		background: var(--tool-selected);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
	.grow {
		flex: 1;
	}
</style>
