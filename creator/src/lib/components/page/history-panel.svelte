<script lang="ts">
	import { untrack } from 'svelte';
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import { confirmDialog } from '@scutoons/ui';
	import { ApiError, unpublish as unpublishEntity, type ContentVersion } from '$lib/api.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { applyBackTo, applyRevert, baseOf, contentOf, describeStep, historyCore, historyFiles, loadFile, planRevert, saved, type Description, type FileHistory, type HistoryFile, type Step } from '$lib/editor/history.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { fileLabel } from '$lib/labels.ts';
	import { NEUTRAL_LOCALE } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { toast } from '$lib/toasts.ts';

	/**
	 * The History (#687): the saved changes of the open item's files as steps – time, who, what in words, before → after –
	 * with Revert (that change, as a new draft) and Back to here, always after a prompt; Unpublish on a live step (sending
	 * out is the bar's Publish alone, RULES H18). A restore saves a new version and posts a History line. Pre-filtered to the open item's
	 * first file, widened to all its files – the bar's History menu.
	 */
	let { doc, docs }: { doc?: ContentDoc; docs?: ContentDoc[] } = $props();

	const session = getCreatorSession();
	const me = $derived(session.user?.email ?? '');
	const all = $derived(docs ?? (doc ? [doc] : []));
	const files = $derived(historyFiles(all));
	let wide = $state(false);
	const shown = $derived(wide ? files : files.slice(0, 1));
	const label = (f: HistoryFile) => `${fileLabel(f.key)}${f.locale !== NEUTRAL_LOCALE && f.doc.locales.length > 1 ? ` · ${f.locale}` : ''}`;

	let histories = $state<FileHistory[]>([]);
	let words = $state<Record<string, Description | null>>({});
	let error = $state<string | null>(null);
	let busy = $state(false);

	type Row = { h: FileHistory; step: Step; versions: ContentVersion[] };
	const rows = $derived(
		histories
			.flatMap((h) => h.steps.map((step) => ({ h, step, versions: h.versions.filter((v) => v.id >= step.first && v.id <= step.last) })))
			.sort((a, b) => b.step.at_ms - a.step.at_ms)
			.slice(0, 60)
	);

	let asked = 0;
	async function load() {
		error = null;
		const mine = ++asked;
		try {
			const next = await Promise.all(shown.map(loadFile));
			if (mine === asked) histories = next;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	$effect(() => {
		void [shown.map((f) => `${f.key}@${baseOf(f)}`).join(), all.map((d) => d.published[d.primary]).join()];
		load();
	});

	// what each step changed, in words: the newest rows (versions never change, so once is enough)
	$effect(() => {
		for (const r of rows.slice(0, 25)) {
			const id = `${r.step.before}-${r.step.last}`;
			if (untrack(() => id in words)) continue;
			words[id] = null;
			describeStep(r.h.file.key, r.step).then((d) => (words[id] = d), () => {});
		}
	});

	const reviewed = $derived(all.some((d) => reviews.needsReview(d.toon)));
	const inReview = $derived(new Set([...(reviews.overview?.to_review ?? []), ...(reviews.overview?.mine ?? [])].filter((r) => r.state === 'pending').map((r) => r.version)));

	const when = (ms: number) => (ms ? new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(ms)) : '');
	const who = (author: string | null) => (!author ? m['creator.history_import']() : author === me ? m['history.you']() : author);
	const lines = (d: Description | null | undefined, max = 8) => (!d ? '' : [...d.lines.slice(0, max).map((l) => `${l.place}: ${l.before || '–'} → ${l.after || '–'}`), ...(d.lines.length > max || d.more ? [m['history.more']({ count: d.lines.length - Math.min(max, d.lines.length) + d.more })] : [])].join('\n'));
	const unsaved = (f: HistoryFile) => (f.doc.dirtyLocales.includes(f.locale) ? `\n\n${m['history.unsaved']()}` : '');

	async function act(work: () => Promise<unknown>) {
		busy = true;
		error = null;
		try {
			await work();
		} catch (e) {
			error = e instanceof ApiError && e.status === 409 ? m['history.conflict']() : e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
			await load();
		}
	}

	const revert = (r: Row) =>
		act(async () => {
			const plan = await planRevert(r.h.file, r.step.before!, r.step.last);
			if (!plan) return;
			if (!plan.applied) {
				await confirmDialog({ title: m['history.revert_title'](), message: m['history.nothing'](), confirmLabel: m['creator.close']() });
				return;
			}
			const d = (await historyCore()).describe(await contentOf(r.h.file.key, plan.base), plan.doc);
			const kept = plan.kept.length ? `\n\n${m['history.kept']({ places: plan.kept.slice(0, 6).join(', ') })}` : '';
			const message = `${m['history.revert_message']({ when: when(r.step.at_ms), who: who(r.step.author) })}\n\n${lines(d)}${kept}${unsaved(r.h.file)}`;
			if (!(await confirmDialog({ title: m['history.revert_title'](), message, confirmLabel: m['history.revert']() }))) return;
			await applyRevert(r.h.file, r.step.last, plan);
			toast(m['history.reverted']({ file: label(r.h.file), when: when(r.step.at_ms) }), { type: 'success', kind: 'history' });
		});

	const backTo = (r: Row) =>
		act(async () => {
			const base = baseOf(r.h.file);
			if (base === null) return;
			const n = (await historyCore()).backTo(saved(r.h.versions), r.step.last, me);
			const others = n.others.length ? `\n\n${m['history.others']({ people: n.others.join(', ') })}` : '';
			const message = `${m['history.back_message']({ when: when(r.step.at_ms), later: n.later })}${others}${unsaved(r.h.file)}`;
			if (!(await confirmDialog({ title: m['history.back_title'](), message, confirmLabel: m['history.back_to']() }))) return;
			await applyBackTo(r.h.file, r.step.last, base);
			toast(m['history.restored']({ file: label(r.h.file), when: when(r.step.at_ms) }), { type: 'success', kind: 'history' });
		});

	const unpublish = (r: Row) =>
		act(async () => {
			if (!(await confirmDialog({ title: m['creator.unpublish_title'](), message: m['creator.unpublish_confirm']({ file: label(r.h.file), locale: r.h.file.locale }), confirmLabel: m['creator.unpublish_label']() }))) return;
			await unpublishEntity(r.h.file.key);
			await r.h.file.doc.load();
			toast(`${m['creator.unpublish_label']()}: ${label(r.h.file)}`, { type: 'success', kind: 'publish' });
		});
</script>

<div class="history" data-testid="history-panel">
	{#if files.length > 1}
		<div class="scope" role="group">
			<button class="chip" class:on={!wide} onclick={() => (wide = false)}>{files[0] ? label(files[0]) : m['history.this_file']()}</button>
			<button class="chip" class:on={wide} onclick={() => (wide = true)} data-testid="history-all">{m['history.all_files']()}</button>
		</div>
	{/if}
	{#if error}<p class="error">{error}</p>{/if}
	{#if !files.length}
		<p class="sc-empty">{m['creator.static_file']()}</p>
	{/if}
	<ol>
		{#each rows as r (`${r.h.file.key}:${r.step.last}`)}
			{@const base = baseOf(r.h.file)}
			{@const live = r.versions.some((v) => v.status === 'published')}
			{@const d = words[`${r.step.before}-${r.step.last}`]}
			<li class:live data-testid="history-step-{r.step.last}">
				<div class="head">
					<span class="when">{when(r.step.at_ms)}</span>
					<span class="who">{who(r.step.author)}</span>
					{#if wide}<span class="tag">{label(r.h.file)}</span>{/if}
					{#if r.step.count > 1}<span class="tag">{m['history.saves']({ count: r.step.count })}</span>{/if}
					{#if r.step.last === base}<span class="tag">{m['creator.history_current']()}</span>{/if}
					{#if live}<span class="tag">{m['creator.state_published']()}</span>{/if}
					{#if inReview.has(r.step.last)}<span class="tag">{m['creator.review_state_pending']()}</span>{/if}
				</div>
				{#each r.versions.filter((v) => v.message) as v (v.id)}<i class="note">{v.message}</i>{/each}
				{#if r.step.before === null}
					<span class="muted">{m['history.created']()}</span>
				{:else if d}
					<ul class="what">
						{#each d.lines.slice(0, 4) as l, i (i)}
							<li><span class="place">{l.place}</span> <s>{l.before}</s> → <span>{l.after}</span></li>
						{/each}
						{#if d.lines.length > 4 || d.more}<li class="muted">{m['history.more']({ count: d.lines.length - Math.min(4, d.lines.length) + d.more })}</li>{/if}
					</ul>
				{/if}
				<div class="acts">
					{#if r.step.before !== null}<button class="ghost" disabled={busy} onclick={() => revert(r)} data-testid="revert-{r.step.last}">{m['history.revert']()}</button>{/if}
					{#if r.step.last !== base}<button class="ghost" disabled={busy} onclick={() => backTo(r)} data-testid="back-to-{r.step.last}">{m['history.back_to']()}</button>{/if}
					{#if !reviewed && live && !r.h.file.entry}
						<button class="ghost" disabled={busy} onclick={() => unpublish(r)}>{m['creator.unpublish_label']()}</button>
					{/if}
				</div>
			</li>
		{/each}
	</ol>
</div>

<style>
	.scope {
		display: flex;
		gap: 0.25rem;
		margin: 0.4rem 0;
	}
	.chip {
		font-size: var(--type-caption-size);
	}
	.chip.on {
		outline: 1px solid var(--tool-text);
	}
	ol,
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	ol > li {
		display: grid;
		gap: 0.15rem;
		padding: 0.35rem 0;
		border-bottom: 1px solid var(--tool-line);
	}
	ol > li.live {
		background: var(--tool-selected);
		padding-inline: 0.3rem;
	}
	.head {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		align-items: baseline;
	}
	.who,
	.muted,
	.note {
		font-size: var(--type-caption-size);
		color: var(--tool-muted);
	}
	.tag {
		font-size: var(--type-caption-size);
		padding: 0 0.3em;
		border: 1px solid var(--tool-text);
	}
	.what {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		overflow-wrap: anywhere;
	}
	.place {
		opacity: 0.75;
	}
	.acts {
		display: flex;
		flex-wrap: wrap;
		gap: 0.2rem;
	}
	.acts button {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.error {
		color: var(--tool-stop);
	}
</style>
