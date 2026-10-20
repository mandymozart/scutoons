<script lang="ts">
	// The Generate panel (#123): the current shot's generations – the approved one first, then the newest – each with
	// its file (generation-slot), who made it with what, its mark (for review / rejected), Approve and Share (the local
	// file into the shared work storage, so the others see it – never the readers' CDN). Starting a generation joins this
	// panel with the server's generators (#121); the variants board is #125.
	import { Icon, Segmented } from '@scutoons/ui';
	import type { GenerationMark } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { approvePatch, placeOf, shotGenerations, type Generation } from '$lib/editor/storyboard-generations.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import GenerationSlot from '$lib/components/storyboard/generation-slot.svelte';

	let { sb }: { sb: StoryboardPage } = $props();

	const list = $derived(sb.current ? shotGenerations(sb.generations, sb.current) : []);
	let busy = $state<number | null>(null);
	const MARKS: { value: GenerationMark; label: string }[] = $derived([
		{ value: 'review', label: m['storyboard.mark_review']() },
		{ value: 'rejected', label: m['storyboard.mark_rejected']() }
	]);

	async function run(g: Generation, job: () => Promise<void>, done?: string) {
		busy = g.id;
		try {
			await job();
			if (done) toast(done, { type: 'success' });
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
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
	{#if sb.generationsError}
		<p class="sc-error">{m['storyboard.gen_error']({ error: sb.generationsError })}</p>
	{:else}
		<p class="muted">{m['storyboard.gen_count']({ count: list.length })}</p>
	{/if}
	{#each list as g (g.id)}
		<article class:approved={g.approved} data-testid="generation-{g.id}">
			<GenerationSlot {sb} {g} />
			<p class="made">{made(g)}{#if placeWord(g)} · {placeWord(g)}{/if}</p>
			{#if g.status === 'failed'}<p class="sc-error">{m['storyboard.gen_failed']({ error: g.error ?? '' })}</p>{/if}
			{#if g.note}<p class="note">{g.note}</p>{/if}
			<div class="actions">
				<Segmented label={m['storyboard.gen_mark']()} options={MARKS} value={g.mark} onchange={(mark) => run(g, () => sb.review(g, { mark }))} onclear={() => run(g, () => sb.review(g, { mark: null }))} />
				<span class="grow"></span>
				{#if placeOf(g, sb.localFiles).at === 'local' && !g.work_path}
					<button type="button" class="secondary" title={m['storyboard.gen_share_hint']()} disabled={busy === g.id} onclick={() => run(g, () => sb.share(g), m['storyboard.gen_shared']())} data-testid="share-{g.id}"><Icon name="upload-cloud" /> {m['storyboard.gen_share']()}</button>
				{/if}
				<Segmented label={m['storyboard.gen_approve']()} options={[{ value: 'approved', icon: 'check', label: m['storyboard.gen_approve'](), title: m['storyboard.gen_approve_hint'](), disabled: busy === g.id || g.status !== 'succeeded' }]} value={g.approved ? 'approved' : null} onchange={() => run(g, () => sb.review(g, approvePatch(g)))} onclear={() => run(g, () => sb.review(g, approvePatch(g)))} testid="approve-{g.id}" />
			</div>
		</article>
	{/each}
</div>

<style>
	.panel {
		display: grid;
		gap: var(--space-1);
		align-content: start;
		padding: var(--space-1);
		box-sizing: border-box;
	}
	p {
		margin: 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.muted,
	.made {
		color: var(--text-muted);
	}
	article {
		display: grid;
		gap: var(--space-05);
		padding-left: var(--space-05);
		border-left: var(--border-width-focus) solid transparent;
	}
	article.approved {
		border-left-color: var(--ink);
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
