<script lang="ts">
	import EmailsNav from '$lib/components/emails-nav.svelte';
	import PageHeader from '$lib/components/page-header.svelte';
	import { page } from '$app/state';
	import { Button, Skeleton, confirmDialog } from '@scutoons/ui';
	import { Status, toonName, type ToonIndex } from '@scutoons/content';
	import { api, library } from '$lib/api.ts';
	import { errorText, minute, number } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Push notifications (agents/research/PWA.md §2b): how many browsers opted in, and "a new chapter is out" – the only
	 * push there is, sent by an admin's tap here, never automatically. A test goes to the admin's own browsers first.
	 * `?toon=&chapter=` prefills the form (a link from the admin's own AI: it never sends – RULES.md "drafts + a link").
	 * Team: the tools' own messages go by themselves (research/PWA-APPS.md §5) – here a test of each kind to the admin's own
	 * browsers in that tool.
	 */
	interface Notice {
		title: string;
		body: string;
		url: string;
	}
	interface Overview {
		configured: boolean;
		subscribers: { total: number; locales: Record<string, number>; apps?: Record<string, number> };
		sends: { id: number; toon: string; chapter: number; test: boolean; recipients: number; delivered: number; gone: number; failed: number; sent_at: string }[];
	}
	interface Draft {
		notification: Record<string, Notice>;
		recipients: number;
		sent_before: number;
		delivered?: number;
		gone?: number;
		failed?: number;
	}

	const toaster = getToaster();
	let overview = $state<Overview | null>(null);
	let toons = $state<ToonIndex[]>([]);
	let chapters = $state<{ number: number; title: string }[]>([]);
	let toon = $state(page.url.searchParams.get('toon') ?? '');
	let chapter = $state(page.url.searchParams.get('chapter') ?? '');
	let draft = $state<Draft | null>(null);
	let error = $state<string | null>(null);
	let busy = $state(false);

	const load = () =>
		api
			.get<Overview>('/admin/push')
			.then((r) => (overview = r))
			.catch((e) => (error = errorText(e)));
	void load();
	library
		.fetchToons()
		.then((list) => (toons = list.filter((t) => t.status === Status.PUBLISHED)))
		.catch((e) => (error = errorText(e)));

	// the toon's published chapters (never coming soon / drafts: those are never announced)
	$effect(() => {
		const id = toon;
		chapters = [];
		if (!id) return;
		library
			.fetchToon(id, getLocale())
			.then((bundle) => {
				if (toon !== id) return;
				chapters = bundle.seasons.flatMap((s) => s.chapters.filter((c) => c.status === Status.PUBLISHED).map((c) => ({ number: c.number, title: c.title ?? '' })));
			})
			.catch((e) => (error = errorText(e)));
	});

	// the preview: a dry run (the texts per language, how many it would reach)
	$effect(() => {
		const body = { toon, chapter: Number(chapter), dry_run: true };
		draft = null;
		if (!toon || chapter === '') return;
		api
			.post<Draft>('/admin/push/new-chapter', body)
			.then((r) => toon === body.toon && Number(chapter) === body.chapter && (draft = r))
			.catch((e) => (error = errorText(e)));
	});

	/** the team's kinds and the tool each lives in (Domain\Push\TeamPush::KINDS) */
	const TEAM_KINDS = [
		{ kind: 'admin.proposals', app: 'Admin', label: () => m['push.kind_proposals']() },
		{ kind: 'creator.reviews', app: 'Creator', label: () => m['push.kind_reviews']() },
		{ kind: 'creator.drafts', app: 'Creator', label: () => m['push.kind_drafts']() },
		{ kind: 'embassy.due', app: 'Embassy', label: () => m['push.kind_embassy']() }
	];

	async function teamTest(kind: string) {
		if (busy) return;
		busy = true;
		try {
			const r = await api.post<{ recipients: number; delivered: number; gone: number; failed: number }>('/admin/push/team-test', { kind });
			toaster.addToast({ message: m['push.sent']({ delivered: r.delivered, recipients: r.recipients, gone: r.gone, failed: r.failed }), type: 'success' });
			await load();
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			busy = false;
		}
	}

	async function send(test: boolean) {
		if (!draft || busy) return;
		const name = draft.notification[getLocale()]?.body ?? `#${chapter}`;
		if (!test) {
			const message = [m['push.send_message'](), draft.sent_before ? m['push.send_again']({ count: draft.sent_before }) : ''].filter(Boolean).join(' ');
			if (!(await confirmDialog({ title: m['push.send_title']({ chapter: name, count: number(draft.recipients) }), message, confirmLabel: m['push.send']() }))) return;
		}
		busy = true;
		try {
			const r = await api.post<Draft>('/admin/push/new-chapter', { toon, chapter: Number(chapter), test });
			if (test && !r.recipients) toaster.addToast({ message: m['push.test_none'](), type: 'error' });
			else toaster.addToast({ message: m['push.sent']({ delivered: r.delivered ?? 0, recipients: r.recipients, gone: r.gone ?? 0, failed: r.failed ?? 0 }), type: 'success' });
			await load();
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			busy = false;
		}
	}
</script>

<PageHeader type={m['nav.group_system']()} title={m['push.title']()} meta={m['push.meta']()} />
<EmailsNav current="push" />

{#if error}<p class="error-box" role="alert">{error}</p>{/if}

{#if !overview}
	<Skeleton variant="table" columns={3} label={m['shell.loading']()} />
{:else}
	{#if !overview.configured}<p class="error-box" role="alert">{m['push.off']()}</p>{/if}
	<p data-testid="push-subscribers">
		{m['push.subscribers']({ count: number(overview.subscribers.total), de: number(overview.subscribers.locales.de ?? 0), en: number(overview.subscribers.locales.en ?? 0) })}
	</p>

	<div class="form">
		<label>
			<span>{m['push.toon']()}</span>
			<select bind:value={toon} onchange={() => (chapter = '')}>
				<option value=""></option>
				{#each toons as t (t.id)}<option value={t.id}>{toonName(t, getLocale())}</option>{/each}
			</select>
		</label>
		<label>
			<span>{m['push.chapter']()}</span>
			<select bind:value={chapter} disabled={!chapters.length}>
				<option value=""></option>
				{#each chapters as c (c.number)}<option value={String(c.number)}>{c.number} · {c.title}</option>{/each}
			</select>
		</label>
	</div>

	{#if draft}
		<h2>{m['push.preview']()}</h2>
		<ul class="previews">
			{#each Object.entries(draft.notification) as [locale, notice] (locale)}
				<li><span>{notice.title}</span> <span class="muted">{notice.body}</span> <small>{locale} · {notice.url}</small></li>
			{/each}
		</ul>
		{#if draft.sent_before}<p class="muted">{m['push.send_again']({ count: draft.sent_before })}</p>{/if}
		<p class="actions">
			<Button variant="secondary" disabled={busy || !overview.configured} onclick={() => send(true)}>{m['push.test']()}</Button>
			<Button disabled={busy || !overview.configured || !draft.recipients} onclick={() => send(false)}>{m['push.send']()} · {number(draft.recipients)}</Button>
		</p>
	{/if}

	<h2>{m['push.team_title']()}</h2>
	<p class="muted" data-testid="push-team-subscribers">
		{m['push.team_hint']({ creator: number(overview.subscribers.apps?.creator ?? 0), admin: number(overview.subscribers.apps?.admin ?? 0), embassy: number(overview.subscribers.apps?.embassy ?? 0) })}
	</p>
	<p class="actions" data-testid="push-team-tests">
		{#each TEAM_KINDS as k (k.kind)}
			<Button variant="secondary" disabled={busy || !overview.configured} onclick={() => teamTest(k.kind)}>{k.app} · {k.label()}</Button>
		{/each}
	</p>

	{#if overview.sends.length}
		<h2>{m['push.history']()}</h2>
		<table class="history">
			<thead><tr><th>{m['push.col_when']()}</th><th>{m['push.col_chapter']()}</th><th>{m['push.col_result']()}</th></tr></thead>
			<tbody>
				{#each overview.sends as s (s.id)}
					<tr>
						<td>{minute(s.sent_at)}</td>
						<td>{s.toon} · {s.chapter}{s.test ? ` (${m['push.test_mark']()})` : ''}</td>
						<td>{s.delivered} / {s.recipients}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
{/if}

<style>
	.form {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		margin: var(--space-15) 0;
	}
	.form label {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		min-width: 12rem;
	}
	.previews {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.previews li {
		display: flex;
		flex-direction: column;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		margin: var(--space-15) 0;
	}
	.history {
		border-collapse: collapse;
	}
	.history th,
	.history td {
		text-align: left;
		padding: var(--space-05) var(--space-1) var(--space-05) 0;
	}
</style>
