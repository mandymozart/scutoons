<script module lang="ts">
	import type { AdminChangesCard } from '@scutoons/api';

	export const isChangesCard = (data: unknown): data is AdminChangesCard => (data as AdminChangesCard | null)?.kind === 'admin_changes';
</script>

<script lang="ts">
	// Changes the Ask agent proposes (tools/mcp admin.ts `propose_changes`, owner 2026-10-05: the agent never writes on
	// its own): before → after, applied only when the admin presses Apply – here, with the admin's session. Catalog
	// entries are written one by one (each a catalog history row with undo; refused when the entry changed since the
	// agent read it), calendar entries go into ONE new draft of `events` (a person publishes it on the Calendar page),
	// an asset file is replaced under its old path (the old bytes stay as an archived version). An applied card is
	// remembered per browser, so a reopened chat doesn't offer to apply it twice.
	import { ApiError, type AdminChange } from '@scutoons/api';
	import { Button, Icon } from '@scutoons/ui';
	import { admin, assets, content } from '$lib/api.ts';
	import { invalidateCatalog } from '$lib/catalog.ts';
	import { CDN_SERVER } from '$lib/config.ts';
	import { errorText } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { card }: { card: AdminChangesCard } = $props();

	const toaster = getToaster();
	const APPLIED = 'scutoons_admin_applied_changes';
	const appliedIds = (): string[] => {
		try {
			return JSON.parse(localStorage.getItem(APPLIED) ?? '[]');
		} catch {
			return [];
		}
	};

	/** per change: '' = not yet, 'ok', or the error */
	// svelte-ignore state_referenced_locally
	let results = $state<string[]>(card.changes.map(() => ''));
	// svelte-ignore state_referenced_locally
	let done = $state(appliedIds().includes(card.id));
	let applying = $state(false);
	/** files the admin picked for asset changes without a source */
	let files = $state<Record<number, File>>({});

	const note = $derived(`Ask: ${card.title}`.slice(0, 200));
	const okCount = $derived(results.filter((r) => r === 'ok').length);
	const hasCatalog = $derived(card.changes.some((c) => c.type === 'catalog'));
	const hasCalendar = $derived(card.changes.some((c) => c.type === 'calendar'));

	const segments = (path: string) => path.split('.').map((s) => (/^\d+$/.test(s) ? Number(s) : s));
	function setPath(data: any, path: string, value: unknown) {
		const parts = segments(path);
		let at = data;
		for (const s of parts.slice(0, -1)) at = at[s];
		const last = parts.at(-1)!;
		if (value === null && !Array.isArray(at)) delete at[last];
		else at[last] = value;
	}

	const isImage = (v: unknown): v is string => typeof v === 'string' && /\.(png|webp|jpe?g|gif|svg|avif)$/i.test(v);
	const src = (path: string) => (/^https?:/.test(path) ? path : `${CDN_SERVER}/${path.replace(/^\/+/, '')}`);
	const show = (v: unknown) => (v === null || v === undefined || v === '' ? m['changes.empty']() : typeof v === 'string' ? v : JSON.stringify(v));
	const kindLabel = (kind: string) => kind.replace(/s$/, '');

	async function applyCatalog(c: Extract<AdminChange, { type: 'catalog' }>) {
		const { entry } = await admin.catalogGet(c.kind, c.id);
		if (entry.updated_at !== c.base) throw new Error(m['changes.changed_since']());
		const data = structuredClone(entry.data);
		for (const f of c.fields) setPath(data, f.path, f.after);
		await admin.catalogPut(c.kind, c.id, data, note);
	}

	async function applyCalendar(indexes: number[]) {
		let list: Record<string, unknown>[] = [];
		let version: number | null = null;
		try {
			const doc = await content.getDraft<Record<string, unknown>[]>('events');
			list = Array.isArray(doc.content) ? structuredClone(doc.content) : [];
			version = doc.version.id;
		} catch (e) {
			if (!(e instanceof ApiError && e.status === 404)) throw e;
		}
		if (version !== card.calendar_base) throw new Error(m['changes.changed_since']());
		for (const i of indexes) {
			const c = card.changes[i];
			if (c.type !== 'calendar') continue;
			if (c.action === 'add') list.push({ ...c.event });
			else for (const f of c.fields) setPath(list[c.index], f.path, f.after);
		}
		// stored by date, newest first (as the Calendar page saves it); a draft – a person publishes it there
		list.sort((a, b) => String(b.date ?? '').localeCompare(String(a.date ?? '')));
		await content.putDraft('events', list, version, note);
	}

	async function applyAsset(c: Extract<AdminChange, { type: 'asset' }>, i: number) {
		let file: Blob | undefined = files[i];
		if (!file && c.source) {
			const res = await fetch(src(c.source));
			if (!res.ok) throw new Error(`${c.source}: ${res.status}`);
			file = await res.blob();
		}
		if (!file) throw new Error(m['changes.asset_missing']());
		await assets.replaceAt(file, c.path, note);
	}

	async function apply() {
		applying = true;
		const calendar: number[] = [];
		for (const [i, c] of card.changes.entries()) {
			if (results[i] === 'ok') continue;
			if (c.type === 'calendar') {
				calendar.push(i);
				continue;
			}
			try {
				if (c.type === 'catalog') await applyCatalog(c);
				else await applyAsset(c, i);
				results[i] = 'ok';
			} catch (e) {
				results[i] = errorText(e);
			}
		}
		if (calendar.length) {
			try {
				await applyCalendar(calendar);
				for (const i of calendar) results[i] = 'ok';
			} catch (e) {
				for (const i of calendar) results[i] = errorText(e);
			}
		}
		if (hasCatalog) invalidateCatalog();
		applying = false;
		if (okCount === card.changes.length) {
			done = true;
			try {
				localStorage.setItem(APPLIED, JSON.stringify([...appliedIds(), card.id].slice(-200)));
			} catch {
				/* storage blocked: the catalog history / the asset show it was applied */
			}
			toaster.addToast({ type: 'success', message: m['changes.applied']() });
		} else toaster.addToast({ type: 'error', message: m['changes.partly']({ done: okCount, total: card.changes.length }) });
	}
</script>

{#snippet value(v: unknown)}
	{#if isImage(v)}<img src={src(v)} alt="" title={v} loading="lazy" />{:else}<span class="value">{show(v)}</span>{/if}
{/snippet}

<div class="changes" data-testid="changes-card">
	<p class="head"><Icon name="edit-3" size="1em" /><span>{m['changes.title']()}</span></p>
	<p class="what">{card.title}</p>
	<ol class="list">
		{#each card.changes as c, i (i)}
			<li class:failed={results[i] && results[i] !== 'ok'}>
				{#if c.type === 'catalog'}
					<p class="target">
						<a href="/catalog/{c.kind}?open={encodeURIComponent(c.id)}">{kindLabel(c.kind)} · {c.label}</a>
						{#if c.label !== c.id}<span class="muted">{c.id}</span>{/if}
					</p>
					{#each c.fields as f (f.path)}
						<p class="field"><span class="path">{f.path}</span>{@render value(f.before)}<Icon name="arrow-right" size="1em" />{@render value(f.after)}</p>
					{/each}
				{:else if c.type === 'calendar' && c.action === 'add'}
					<p class="target"><a href="/calendar">{m['changes.calendar_new']()}</a></p>
					<p class="field"><span class="value">{c.event.title} · {String(c.event.date ?? '').replace('T', ' ').slice(0, 16)} · {c.event.city}{c.event.location ? `, ${c.event.location}` : ''}</span></p>
				{:else if c.type === 'calendar'}
					<p class="target"><a href="/calendar">{m['changes.calendar_entry']()} · {c.label}</a></p>
					{#each c.fields as f (f.path)}
						<p class="field"><span class="path">{f.path}</span>{@render value(f.before)}<Icon name="arrow-right" size="1em" />{@render value(f.after)}</p>
					{/each}
				{:else}
					<p class="target"><a href="/assets?q={encodeURIComponent(c.path)}">{m['changes.asset']()}</a> <span class="muted">{m['changes.asset_used']({ count: c.usages })}</span></p>
					<p class="field">
						{@render value(c.path)}<Icon name="arrow-right" size="1em" />
						{#if c.source}{@render value(c.source)}
						{:else if files[i]}<span class="value">{files[i].name}</span>
						{:else}<span class="muted">?</span>{/if}
					</p>
					<p class="path">{c.path}</p>
					{#if !c.source && !done && results[i] !== 'ok'}
						<label class="pick">
							<span>{m['changes.asset_pick']()}</span>
							<input type="file" accept="image/*,video/*,audio/*" onchange={(e) => {
								const file = e.currentTarget.files?.[0];
								if (file) files[i] = file;
							}} />
						</label>
					{/if}
				{/if}
				{#if results[i] === 'ok'}<p class="status"><Icon name="check" size="1em" /> {m['changes.applied']()}</p>
				{:else if results[i]}<p class="sc-error">{results[i]}</p>{/if}
			</li>
		{/each}
	</ol>
	<div class="row">
		{#if done}
			<span class="caption"><Icon name="check" size="1em" /> {m['changes.applied']()}{#if hasCatalog}{' · '}{m['changes.undo_catalog']()}{/if}{#if hasCalendar}{' · '}{m['changes.calendar_draft']()}{/if}</span>
		{:else}
			<span class="caption muted">{m['changes.note']()}</span>
			<Button onclick={apply} disabled={applying} data-testid="changes-apply">{applying ? m['changes.applying']() : m['changes.apply']()}</Button>
		{/if}
	</div>
</div>

<style>
	.changes {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin-top: var(--space-1);
		padding: var(--space-1);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin: 0;
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.what {
		margin: 0;
	}
	.list {
		display: flex;
		flex-direction: column;
		margin: 0;
		padding: 0;
		list-style: none;
		max-height: 24rem;
		overflow-y: auto;
	}
	.list li {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		padding: var(--space-05) 0;
		border-top: var(--border-width-hairline) solid var(--border-subtle);
	}
	.list li.failed {
		border-left: var(--border-width-regular) solid var(--state-error);
		padding-left: var(--space-05);
	}
	.list p {
		margin: 0;
	}
	.target a {
		color: var(--ink);
	}
	.field {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-025) var(--space-05);
		overflow-wrap: anywhere;
	}
	.path,
	.muted {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.field .path {
		flex-basis: 100%;
	}
	.field img {
		width: 3rem;
		height: 3rem;
		object-fit: contain;
		border: var(--border-width-hairline) solid var(--border-subtle);
		background: var(--paper);
	}
	.pick {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		font-size: var(--type-caption-size);
	}
	.status,
	.caption {
		font-size: var(--type-caption-size);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
	}
</style>
