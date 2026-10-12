<script lang="ts">
	import type { Component, Snippet } from 'svelte';
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Button, DataTable, Field, Sheet, Skeleton, SplitView, ToonAvatar, confirmDialog, listHref, type Column } from '@scutoons/ui';
	import { mediaUrl } from '@scutoons/media';
	import { ApiError, type CatalogEntry } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { invalidateCatalog } from '$lib/catalog.ts';
	import { restOf, withRest, type CatalogFormProps, type CatalogLists, type Data, type KindDef } from '@scutoons/catalog';
	import { toonAvatarUrl } from '$lib/config.ts';
	import { errorText } from '$lib/format.ts';
	import { infoBar } from '$lib/info.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHelp from '$lib/components/page-help.svelte';
	import CatalogHistory from '$lib/components/catalog/catalog-history.svelte';
	import { CatalogProblems, JsonField } from '@scutoons/catalog';
	import { ACCEPTED_WARNINGS_FIELD, type CharacterRegistry } from '@scutoons/content';
	import { canAccept, catalogProblems, entryOf, isAccepted, withAccepted, type CatalogProblem } from '@scutoons/catalog';


	/**
	 * The one catalog scaffold every kind uses (agents/CATALOG.md part 3): the list in its order (the grip – the shared Sortable – = POST reorder),
	 * a click opens the entry's form next to it (SplitView; `?open=<id>`), "New" a blank one (`?new=1`); Save = PUT (422
	 * problems listed above the form and at the fields), Delete asks in-page and shows what still uses it (409 in_use);
	 * the entry's history under the form and the kind's history in a drawer, both with undo. The kind's own fields are
	 * its `form` component; the keys it does not show are the "Advanced" JSON, so nothing is uneditable.
	 */
	let {
		def,
		lists,
		registry = null,
		refresh,
		form: Form,
		barActions,
		detailActions,
		reload = 0,
		readonly = false
	}: {
		/** the kind is edited in the creator now (owner 2026-10-12, D6): no new / save / delete, the form only shows (the order stays) */
		readonly?: boolean;
		/** bumped by the page after it wrote entries itself (the open entry is read again) */
		reload?: number;
		def: KindDef;
		lists: CatalogLists;
		/** the published character registry (wear overlays are checked against it); null = not loaded, not checked */
		registry?: CharacterRegistry | null;
		/** reads a kind's list again (after a write) */
		refresh: (kind: KindDef['kind']) => Promise<void>;
		form: Component<CatalogFormProps, {}, 'data'>;
		/** more buttons in the page's ActionBar (e.g. the items' "Import images …") */
		barActions?: Snippet;
		/** more buttons in the open entry's header (e.g. a toon's "Fill it in the creator") */
		detailActions?: Snippet<[string]>;
	} = $props();

	const toaster = getToaster();
	const kind = $derived(def.kind);
	const entries = $derived(lists[kind] ?? null);
	const openId = $derived(page.url.searchParams.get('open'));
	const creating = $derived(page.url.searchParams.has('new'));

	type Shown = 'none' | 'entry' | 'new' | 'missing';
	let shown = $state<Shown>('none');
	let draft = $state<Data>({});
	let original = $state('');
	let newId = $state('');
	let problems = $state<string[]>([]);
	let uses = $state<string[]>([]);
	let saving = $state(false);
	let moving = $state(false);
	let historyReload = $state(0);
	let kindHistoryOpen = $state(false);

	const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value ?? null));
	const dirty = $derived(shown !== 'none' && shown !== 'missing' && (JSON.stringify(draft) !== original || (shown === 'new' && newId !== '')));

	function select(key: string | null, isNew: boolean) {
		problems = [];
		uses = [];
		if (isNew) {
			draft = def.blank();
			newId = '';
			shown = 'new';
		} else if (key === null) {
			shown = 'none';
			draft = {};
		} else {
			const entry = (entries ?? []).find((e) => e.id === key);
			shown = entry ? 'entry' : 'missing';
			draft = entry ? clone(def.toForm ? def.toForm(entry.data) : entry.data) : {};
		}
		original = JSON.stringify(draft);
	}

	// the entry of the URL, once the list is there (a list refresh alone does not reset the form)
	$effect(() => {
		const key = openId;
		const isNew = creating;
		if (entries === null) return;
		void def;
		untrack(() => select(key, isNew));
	});
	// the page changed entries itself (the items' image import): the open entry again from the list, unless it has edits
	$effect(() => {
		if (!reload) return;
		untrack(() => {
			if (shown === 'entry' && openId && !dirty) select(openId, false);
		});
	});

	const nav = (patch: Record<string, string | number | null>) => goto(listHref(page.url, patch), { keepFocus: true, noScroll: true });

	async function leave(): Promise<boolean> {
		return !dirty || confirmDialog({ title: m['catalog.discard_title'](), message: m['catalog.discard_message'](), confirmLabel: m['catalog.discard']() });
	}
	async function openEntry(id: string) {
		if (id === openId && !creating) return;
		if (await leave()) nav({ open: id, new: null });
	}
	async function openNew() {
		if (await leave()) nav({ open: null, new: 1 });
	}
	async function close() {
		if (await leave()) nav({ open: null, new: null });
	}

	const currentId = $derived(shown === 'new' ? newId.trim() : (openId ?? ''));
	const title = $derived(
		shown === 'new' ? m['catalog.new_entry']({ what: def.singular() }) : shown === 'entry' && openId ? def.title(def.fromForm ? def.fromForm(draft) : draft, openId) : (openId ?? '')
	);

	function idProblem(id: string): string | null {
		if (!def.idPattern.test(id)) return m['catalog.id_invalid']();
		if ((entries ?? []).some((e) => e.id === id)) return m['catalog.id_taken']();
		return null;
	}
	const newIdError = $derived(shown === 'new' && newId !== '' ? idProblem(newId.trim()) : null);

	async function save() {
		if (shown !== 'entry' && shown !== 'new') return;
		const id = currentId;
		if (shown === 'new') {
			const p = idProblem(id);
			if (p) {
				problems = [`id: ${p}`];
				return;
			}
		}
		const form = clone(draft);
		let data = def.fromForm ? def.fromForm(form) : form;
		if (def.dataId && data && typeof data === 'object') {
			const { id: _old, ...rest } = data as Data;
			data = { id, ...rest };
		}
		if (def.confirmSave && !(await confirmDialog({ title: m['catalog.confirm_save_title']({ id }), message: m['catalog.confirm_save_types'](), confirmLabel: m['catalog.save']() }))) return;
		saving = true;
		problems = [];
		uses = [];
		try {
			const result = await admin.catalogPut(kind, id, data);
			invalidateCatalog();
			await refresh(kind);
			historyReload++;
			toaster.addToast({
				message: !result.changed ? m['catalog.unchanged']() : result.created ? m['catalog.created']({ id }) : m['catalog.saved']({ id }),
				type: result.changed ? 'success' : 'info'
			});
			if (shown === 'new') {
				original = JSON.stringify(draft);
				newId = '';
				await nav({ new: null, open: id });
			} else select(id, false);
		} catch (e) {
			if (e instanceof ApiError && e.code === 'invalid_catalog') problems = (e.body?.problems as string[] | undefined) ?? [e.message];
			else toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			saving = false;
		}
	}

	async function remove() {
		const id = openId;
		if (!id || shown !== 'entry') return;
		const ok = await confirmDialog({
			title: m['catalog.delete_title']({ what: def.singular(), id }),
			message: m['catalog.delete_message'](),
			confirmLabel: m['catalog.delete']()
		});
		if (!ok) return;
		saving = true;
		uses = [];
		try {
			await admin.catalogDelete(kind, id);
			invalidateCatalog();
			await refresh(kind);
			toaster.addToast({ message: m['catalog.deleted']({ id }), type: 'success' });
			original = JSON.stringify(draft);
			await nav({ open: null });
		} catch (e) {
			if (e instanceof ApiError && e.code === 'in_use') uses = (e.body?.uses as string[] | undefined) ?? [e.message];
			else toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			saving = false;
		}
	}

	async function move(from: number, to: number) {
		if (!entries || moving || from === to) return;
		const ids = entries.map((e) => e.id);
		const [id] = ids.splice(from, 1);
		ids.splice(to, 0, id);
		moving = true;
		try {
			await admin.catalogReorder(kind, ids);
			invalidateCatalog();
			await refresh(kind);
			historyReload++;
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			moving = false;
		}
	}

	/** after an undo: the list and the open entry from the server */
	async function undone() {
		invalidateCatalog();
		await refresh(kind);
		historyReload++;
		if (shown === 'entry' && openId) select(openId, false);
	}

	const columns = $derived<Column[]>([
		...def.columns.map((c) => ({ id: c.id, label: c.label(), secondary: c.secondary, width: c.image || c.id === 'avatar' ? '3.5rem' : undefined })),
	]);
	const columnDef = (id: string) => def.columns.find((c) => c.id === id);
	// accepted warnings are kept out of the Advanced JSON: the Problems panel is their one control
	const advancedKeys = $derived(def.formKeys ? [...def.formKeys, ACCEPTED_WARNINGS_FIELD] : null);
	const advanced = $derived(advancedKeys && shown !== 'none' && shown !== 'missing' ? restOf(draft, advancedKeys) : null);

	// ── Problems (validateCatalog over the lists; a warning accepted in its entry – lib/catalog/problems.ts) ─────────
	let problemsOpen = $state(false);
	let accepting = $state<CatalogProblem | null>(null);
	/** the lists as the editor stands: the open entry as its form has it (problems follow the typing) */
	const liveLists = $derived<CatalogLists>(
		shown === 'entry' && openId && lists[kind]
			? { ...lists, [kind]: lists[kind]!.map((e) => (e.id === openId ? { ...e, data: (def.fromForm ? def.fromForm(draft) : draft) as Data } : e)) }
			: lists
	);
	const kindProblems = $derived(catalogProblems(liveLists, registry).filter((p) => p.list === kind));
	/** the problem's entry is the one open in the form */
	const inDraft = (p: CatalogProblem) => shown === 'entry' && !!p.place && p.place.kind === kind && p.place.id === openId;
	const accepted = (p: CatalogProblem) => isAccepted(liveLists, p);
	const problemCounts = $derived({
		error: kindProblems.filter((p) => p.level === 'error').length,
		warning: kindProblems.filter((p) => p.level === 'warning' && !accepted(p)).length
	});

	async function openProblem(p: CatalogProblem) {
		if (!p.place || p.place.kind !== kind) return;
		problemsOpen = false;
		await openEntry(p.place.id);
	}

	/** ✓: the open entry takes it into its form (unsaved, Save keeps it – like any edit of it); another entry is saved at
	 *  once (one change in its history, undo takes it back) */
	async function toggleAccepted(p: CatalogProblem) {
		if (!canAccept(liveLists, p) || !p.place || accepting) return;
		const on = !accepted(p);
		if (inDraft(p)) {
			draft = withAccepted(draft, p, on);
			return;
		}
		const entry = entryOf(lists, p.place);
		if (!entry) return;
		accepting = p;
		try {
			const kindOf = p.place.kind;
			await admin.catalogPut(kindOf, entry.id, withAccepted(entry.data, p, on), on ? m['catalog.problems_accepted_note']() : m['catalog.problems_unaccepted_note']());
			await refresh(kindOf);
			historyReload++;
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			accepting = null;
		}
	}
</script>

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['catalog.title']()}>
	{#snippet info()}<PageHelp page="catalog" />{/snippet}
	{#snippet start()}
		{#if entries}<span class="muted">{m['catalog.count']({ count: entries.length, what: def.label() })}</span>{/if}
		{#if dirty}<span>{m['catalog.unsaved']()}</span>{/if}
	{/snippet}
	{#snippet secondary()}
		{#if def.kind === 'items' || def.kind === 'albums' || def.kind === 'recipes'}
			<Button size="sm" variant="secondary" onclick={() => (problemsOpen = true)} data-testid="catalog-problems-toggle">
				{m['catalog.problems_button']()} <span class:bad={problemCounts.error > 0}>{problemCounts.error}</span>/<span>{problemCounts.warning}</span>
			</Button>
		{/if}
		<Button size="sm" variant="secondary" onclick={() => (kindHistoryOpen = true)} data-testid="catalog-kind-history">{m['catalog.history']()}</Button>
		<!-- the page's own actions (items: Import images …) – in "More" when narrow: the bar's end pushed them under More on a
		     phone (E.9); wide, the secondary actions sit right before the end, so the order stays -->
		{#if barActions}{@render barActions()}{/if}
	{/snippet}
	{#snippet end()}
		{#if def.creatable && !readonly}
			<Button size="sm" onclick={openNew} data-testid="catalog-new">{m['catalog.new']({ what: def.singular() })}</Button>
		{/if}
	{/snippet}
</ActionBar>

<SplitView open={shown !== 'none'} {title} closeLabel={m['common.close']()} foldLabel={m['shell.fold_panel']()} maximizeLabel={m['shell.maximize_panel']()} restoreLabel={m['shell.restore_panel']()} onclose={close}>
	{#snippet list()}
		{#if !entries}
			<Skeleton variant="table" columns={4} label={m['shell.loading']()} />
		{:else}
			<DataTable
				{columns}
				rows={entries}
				rowKey={(e: CatalogEntry) => e.id}
				activeKey={creating ? null : openId}
				onrowclick={(e: CatalogEntry) => openEntry(e.id)}
				labels={tableLabels()}
				onmove={def.reorderable ? move : undefined}
				moveLabel={(id) => m['common.move']({ name: String(entries?.find((e) => e.id === id)?.data?.name ?? id) })}
				moveHint={m['common.move_hint']()}
				moveDisabled={moving}
			>
				{#snippet cell(entry: CatalogEntry, column: Column)}
					{@const c = columnDef(column.id)}
					{#if column.id === 'avatar'}
						<ToonAvatar src={toonAvatarUrl(entry.id)} name={String(entry.data?.name ?? entry.id)} />
					{:else if c?.image}
						{@const path = c.image(entry.data, lists)}
						{#if path}<img class="thumb" src={mediaUrl(path)} alt="" loading="lazy" />{/if}
					{:else if c?.text}
						{c.text(entry.data, lists, entry)}
					{/if}
				{/snippet}
			</DataTable>
		{/if}
	{/snippet}
	{#snippet actions()}
		{#if shown === 'entry' && openId && detailActions}{@render detailActions(openId)}{/if}
		{#if shown === 'entry' && def.deletable && !readonly}
			<Button size="sm" variant="secondary" onclick={remove} disabled={saving} data-testid="catalog-delete">{m['catalog.delete']()}</Button>
		{/if}
		{#if (shown === 'entry' || shown === 'new') && !readonly}
			<Button size="sm" onclick={save} disabled={saving || (shown === 'new' && (!newId.trim() || !!newIdError))} data-testid="catalog-save">
				{saving ? m['catalog.saving']() : m['catalog.save']()}
			</Button>
		{/if}
	{/snippet}
	{#snippet detail()}
		{#if shown === 'missing'}
			<p class="error-box" role="alert">{m['catalog.not_found']({ id: openId ?? '' })}</p>
		{:else if shown !== 'none'}
			<form
				class="entry-form"
				data-testid="catalog-form"
				data-kind={kind}
				onsubmit={(event) => {
					event.preventDefault();
					void save();
				}}
			>
				{#if uses.length}
					<div class="error-box" role="alert" data-testid="catalog-in-use">
						<p>{m['catalog.in_use']({ id: openId ?? '' })}</p>
						<ul>{#each uses as use (use)}<li>{use}</li>{/each}</ul>
					</div>
				{/if}
				{#if problems.length}
					<div class="error-box" role="alert" data-testid="catalog-problems">
						<p>{m['catalog.problems']()}</p>
						<ul>{#each problems as p (p)}<li>{p}</li>{/each}</ul>
					</div>
				{/if}
				{#if shown === 'new'}
					<Field label="id" required error={newIdError} hint={m['catalog.id_hint']()}>
						{#snippet children({ id, describedby, invalid })}
							<input {id} aria-describedby={describedby} aria-invalid={invalid} bind:value={newId} autocomplete="off" spellcheck="false" data-testid="catalog-new-id" />
						{/snippet}
					</Field>
				{:else}
					<p class="sc-caption muted">id: <span class="mono-code">{openId}</span></p>
				{/if}
				<fieldset class="plain" disabled={readonly}>
					<Form bind:data={draft} creating={shown === 'new'} id={currentId} {lists} {problems} />
				</fieldset>
				{#if advanced}
					<details class="advanced" open={Object.keys(advanced).length > 0 && problems.length > 0}>
						<summary>{m['catalog.advanced']()}</summary>
						<JsonField
							label={m['catalog.advanced_json']()}
							hint={m['catalog.advanced_hint']()}
							value={advanced}
							onchange={(rest) => (draft = withRest(draft, def.formKeys ?? [], rest))}
							testid="catalog-advanced"
						/>
					</details>
				{/if}
				<button type="submit" hidden>{m['catalog.save']()}</button>
			</form>
			{#if shown === 'entry' && openId}
				<details class="entry-history">
					<summary>{m['catalog.history']()}</summary>
					<CatalogHistory {kind} id={openId} reload={historyReload} onundone={undone} {readonly} />
				</details>
			{/if}
		{/if}
	{/snippet}
</SplitView>

<Sheet bind:open={kindHistoryOpen} title={m['catalog.history_of']({ what: def.label() })} side="right" widthKey="history" closeLabel={m['common.close']()} resizeLabel={m['shell.resize_panel']()}>
	{#if kindHistoryOpen}<CatalogHistory {kind} reload={historyReload} onundone={undone} {readonly} />{/if}
</Sheet>

<Sheet bind:open={problemsOpen} title={m['catalog.problems_title']({ what: def.label() })} side="right" widthKey="problems" closeLabel={m['common.close']()} resizeLabel={m['shell.resize_panel']()}>
	{#if problemsOpen}<CatalogProblems problems={kindProblems} lists={liveLists} {accepted} busy={accepting} onopen={openProblem} ontoggle={toggleAccepted} />{/if}
</Sheet>

<style>
	.bad {
		color: var(--state-error);
	}
	.thumb {
		display: block;
		width: 2.5rem;
		height: 2.5rem;
		object-fit: contain;
	}
	.entry-form {
		display: flex;
		flex-direction: column;
	}
	.entry-form .error-box {
		margin-bottom: var(--space-1);
	}
	.error-box p,
	.error-box ul {
		margin: 0;
	}
	.error-box ul {
		padding-left: 1.2em;
	}
	.advanced,
	.entry-history {
		margin-top: var(--space-1);
	}
	p {
		margin: 0 0 var(--space-1);
	}
	/* a read-only kind's form (D6): the fieldset only disables, it draws nothing */
	.plain {
		border: 0;
		margin: 0;
		padding: 0;
		min-width: 0;
	}
</style>
