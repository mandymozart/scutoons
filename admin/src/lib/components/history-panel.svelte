<script lang="ts">
	import { untrack } from 'svelte';
	import { Button, confirmDialog } from '@scutoons/ui';
	import { ApiError, type ContentVersion, type KeyHistory } from '@scutoons/api';
	import type { HistoryDescription, HistoryStep } from '@scutoons/content';
	import { content } from '$lib/api.ts';
	import { history } from '$lib/history.ts';
	import { errorText } from '$lib/format.ts';
	import { getAdminSession } from '$lib/session.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The History (#687) of one stored document: its saved changes as steps – time, who, what in words, before → after –
	 * with Revert (that change, as a new draft) and Back to here, always after a prompt; Publish for a step that is not
	 * live. The rules are deck-core's (lib/history.ts); `base` = the version the page stands on, `onchange` reloads the page.
	 */
	let { docKey, base, onchange }: { docKey: string; base: number | null; onchange: () => Promise<void> | void } = $props();

	const session = getAdminSession();
	const me = $derived(session.user?.email ?? '');

	let loaded = $state<KeyHistory | null>(null);
	let words = $state<Record<string, HistoryDescription | null>>({});
	let error = $state<string | null>(null);
	let busy = $state(false);

	const rows = $derived(
		(loaded?.steps ?? [])
			.map((step) => ({ step, versions: (loaded?.versions ?? []).filter((v: ContentVersion) => v.id >= step.first && v.id <= step.last) }))
			.sort((a, b) => b.step.at_ms - a.step.at_ms)
			.slice(0, 40)
	);

	let asked = 0;
	async function load() {
		error = null;
		const mine = ++asked;
		try {
			const next = await history.load(docKey);
			if (mine === asked) loaded = next;
		} catch (e) {
			error = errorText(e);
		}
	}

	$effect(() => {
		void [docKey, base];
		load();
	});

	// what each step changed, in words (versions never change, so once is enough)
	$effect(() => {
		for (const { step } of rows.slice(0, 20)) {
			const id = `${step.before}-${step.last}`;
			if (untrack(() => id in words)) continue;
			words[id] = null;
			history.describe(docKey, step).then((d) => (words[id] = d), () => {});
		}
	});

	const when = (ms: number) => (ms ? new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(ms)) : '');
	const who = (author: string | null) => (!author ? m['history.import']() : author === me ? m['history.you']() : author);
	const lines = (d: HistoryDescription, max = 8) =>
		[...d.lines.slice(0, max).map((l) => `${l.place}: ${l.before || '–'} → ${l.after || '–'}`), ...(d.lines.length > max || d.more ? [m['history.more']({ count: d.lines.length - Math.min(max, d.lines.length) + d.more })] : [])].join('\n');

	async function act(work: () => Promise<unknown>) {
		busy = true;
		error = null;
		try {
			await work();
			await onchange();
		} catch (e) {
			error = e instanceof ApiError && e.status === 409 ? m['history.conflict']() : errorText(e);
		} finally {
			busy = false;
			await load();
		}
	}

	const revert = (step: HistoryStep) =>
		act(async () => {
			if (base === null || step.before === null) return;
			const plan = await history.plan(docKey, step.before, step.last, base);
			if (!plan.applied) {
				await confirmDialog({ title: m['history.revert_title'](), message: m['history.nothing'](), confirmLabel: m['history.close']() });
				return;
			}
			const d = (await history.core()).describe(await history.contentOf(docKey, base), plan.doc);
			const kept = plan.kept.length ? `\n\n${m['history.kept']({ places: plan.kept.slice(0, 6).join(', ') })}` : '';
			const message = `${m['history.revert_message']({ when: when(step.at_ms), who: who(step.author) })}\n\n${lines(d)}${kept}`;
			if (!(await confirmDialog({ title: m['history.revert_title'](), message, confirmLabel: m['history.revert']() }))) return;
			await history.apply(docKey, step.last, plan);
		});

	const backTo = (step: HistoryStep) =>
		act(async () => {
			if (base === null || !loaded) return;
			const n = await history.backToWords(loaded.versions, step.last, me);
			const others = n.others.length ? `\n\n${m['history.others']({ people: n.others.join(', ') })}` : '';
			const message = `${m['history.back_message']({ when: when(step.at_ms), later: n.later })}${others}`;
			if (!(await confirmDialog({ title: m['history.back_title'](), message, confirmLabel: m['history.back_to']() }))) return;
			await history.backTo(docKey, step.last, base);
		});

	const publish = (step: HistoryStep) =>
		act(async () => {
			if (!(await confirmDialog({ title: m['history.publish_title'](), message: m['history.publish_message']({ when: when(step.at_ms) }), confirmLabel: m['history.publish']() }))) return;
			await content.publishVersion(docKey, step.last);
		});
</script>

<div class="history" data-testid="history-panel">
	{#if error}<p class="error-box" role="alert">{error}</p>{/if}
	<ol>
		{#each rows as { step, versions } (step.last)}
			{@const live = versions.some((v) => v.status === 'published')}
			{@const d = words[`${step.before}-${step.last}`]}
			<li class:live data-testid="history-step-{step.last}">
				<div class="head">
					<span>{when(step.at_ms)}</span>
					<span class="muted">{who(step.author)}</span>
					{#if step.count > 1}<span class="tag">{m['history.saves']({ count: step.count })}</span>{/if}
					{#if step.last === base}<span class="tag">{m['history.current']()}</span>{/if}
					{#if live}<span class="tag">{m['history.published']()}</span>{/if}
				</div>
				{#each versions.filter((v) => v.message) as v (v.id)}<i class="muted">{v.message}</i>{/each}
				{#if step.before === null}
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
					{#if step.before !== null}<Button size="sm" variant="secondary" disabled={busy || base === null} onclick={() => revert(step)} data-testid="revert-{step.last}">{m['history.revert']()}</Button>{/if}
					{#if step.last !== base}<Button size="sm" variant="secondary" disabled={busy || base === null} onclick={() => backTo(step)} data-testid="back-to-{step.last}">{m['history.back_to']()}</Button>{/if}
					{#if !live}<Button size="sm" variant="secondary" disabled={busy} onclick={() => publish(step)}>{m['history.publish']()}</Button>{/if}
				</div>
			</li>
		{/each}
	</ol>
</div>

<style>
	ol,
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	ol > li {
		display: grid;
		gap: 0.15rem;
		padding: 0.5rem 0;
		border-bottom: 1px solid var(--border-subtle, #ddd);
	}
	ol > li.live {
		background: var(--surface-muted, #eee);
		padding-inline: 0.5rem;
	}
	.head {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		align-items: baseline;
	}
	.muted {
		color: var(--text-muted);
	}
	.tag {
		padding: 0 0.3em;
		border: 1px solid currentColor;
	}
	.what {
		overflow-wrap: anywhere;
	}
	.place {
		opacity: 0.75;
	}
	.acts {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
</style>
