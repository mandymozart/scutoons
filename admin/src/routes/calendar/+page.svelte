<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { ActionBar, Button, CloseIcon, Section, Skeleton } from '@scutoons/ui';
	import { ApiError, type WrappedContent } from '@scutoons/api';
	import { EventCategory, type Event } from '@scutoons/content';
	import { content } from '$lib/api.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { errorText, minute } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/**
	 * The public events list (viewer /events): the platform-wide content entity `events` (entity_versions;
	 * production's events entity is the source). Saving stores a new version and publishes it; a stored draft that is newer (the
	 * calendar changes from a proposal are drafts) is published with Publish. The viewer splits upcoming / past.
	 * Fields the form doesn't know (e.g. `toon`) are kept as they are.
	 */
	const KEY = 'events';
	const toaster = getToaster();
	const locale = getLocale();

	let stored = $state<WrappedContent<Event[]> | null>(null);
	let loaded = $state(false);
	/** the list being edited (stable ids for the keyed each) */
	let draft = $state<{ id: number; event: Event }[]>([]);
	let saved = $state('[]');
	let error = $state<string | null>(null);
	let saving = $state(false);
	let nextId = 0;

	const categories = [
		{ value: EventCategory.IRL, label: () => m['calendar.cat_irl']() },
		{ value: EventCategory.ONLINE, label: () => m['calendar.cat_virtual']() },
		{ value: EventCategory.HYBRID, label: () => m['calendar.cat_hybrid']() }
	];

	async function load() {
		error = null;
		try {
			apply(await content.getDraft<Event[]>(KEY));
		} catch (e) {
			if (e instanceof ApiError && e.status === 404) apply(null);
			else error = errorText(e);
		}
	}

	function apply(next: WrappedContent<Event[]> | null) {
		stored = next;
		const list = Array.isArray(next?.content) ? next.content : [];
		draft = list.map((event) => ({ id: nextId++, event: { ...event } }));
		saved = JSON.stringify(list);
		loaded = true;
	}

	load();

	const list = () => draft.map((d) => $state.snapshot(d.event) as Event);
	const changed = $derived(loaded && JSON.stringify(draft.map((d) => d.event)) !== saved);
	/** a stored draft newer than the published list (e.g. from an applied changes card): Publish makes it live */
	const unpublished = $derived(!!stored && stored.version.status !== 'published');
	const now = Date.now();
	const upcoming = $derived(draft.filter((d) => !d.event.date || Date.parse(d.event.date) > now).sort((a, b) => (a.event.date || '￿').localeCompare(b.event.date || '￿')));
	const past = $derived(draft.filter((d) => d.event.date && Date.parse(d.event.date) <= now).sort((a, b) => b.event.date.localeCompare(a.event.date)));

	function add() {
		draft = [{ id: nextId++, event: { title: '', date: '', city: '', location: '', category: EventCategory.IRL } }, ...draft];
	}

	function set(id: number, field: keyof Event, value: string) {
		const row = draft.find((d) => d.id === id);
		if (!row) return;
		if (field === 'link' && !value) delete row.event.link;
		else (row.event as unknown as Record<string, string>)[field] = value;
	}

	async function save() {
		const events = list();
		if (events.some((e) => !e.date || !e.title.trim() || !e.city.trim())) {
			toaster.addToast({ message: m['calendar.invalid'](), type: 'error' });
			return;
		}
		// stored by date, newest first (the viewer sorts again)
		events.sort((a, b) => b.date.localeCompare(a.date));
		saving = true;
		try {
			const id = !changed && stored ? stored.version.id : (await content.putDraft(KEY, events, stored?.version.id ?? null, 'admin: events')).version.id;
			await content.publishVersion(KEY, id);
			apply(await content.getDraft<Event[]>(KEY));
			toaster.addToast({ message: m['calendar.saved'](), type: 'success' });
		} catch (e) {
			if (e instanceof ApiError && e.status === 409) {
				toaster.addToast({ message: m['calendar.conflict'](), type: 'error' });
				await load();
			} else toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			saving = false;
		}
	}

	/** `datetime-local` wants `YYYY-MM-DDTHH:MM`; stored with seconds like the seed */
	const inputDate = (value: string) => (value ? value.slice(0, 16) : '');
	const storedDate = (value: string) => (value ? `${value.slice(0, 16)}:00` : '');
	const eventsPage = `${VIEWER_URL}/${locale === 'de' ? '' : `${locale}/`}events`;
</script>

<PageHeader type={m['nav.group_data']()} title={m['calendar.title']()} meta={m['calendar.meta']()} />

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['calendar.title']()}>
	{#snippet info()}<PageHelp page="calendar" />{/snippet}
	{#snippet start()}
		{#if changed}<span>{m['calendar.unsaved']()}</span>
		{:else if loaded}
			<span class="muted">
				{#if !stored}{m['calendar.none_stored']()}
				{:else if stored.version.status !== 'published'}{m['calendar.draft_newer']({ at: minute(stored.version.created_at) })}
				{:else}{m['calendar.last_change']({ at: minute(stored.version.created_at), by: stored.version.author_email ?? stored.version.author ?? '—' })}{/if}
			</span>
		{/if}
	{/snippet}
	{#snippet end()}
		<Button size="sm" variant="secondary" href={eventsPage} target="_blank" rel="noopener">{m['calendar.preview']()}</Button>
		<Button size="sm" variant="secondary" onclick={add} disabled={!loaded} data-testid="events-add">{m['calendar.add']()}</Button>
		<Button size="sm" onclick={save} disabled={!loaded || saving || !(changed || unpublished)} data-testid="events-save">
			{saving ? m['calendar.saving']() : changed ? m['calendar.save']() : unpublished ? m['calendar.publish']() : m['calendar.save']()}
		</Button>
	{/snippet}
</ActionBar>

{#snippet rows(items: { id: number; event: Event }[])}
	<ul class="events">
		{#each items as { id, event } (id)}
			<li class="row" data-testid="event-row">
				<div class="fields">
					<label>
						<span class="muted">{m['calendar.date']()}</span>
						<input type="datetime-local" required value={inputDate(event.date)} onchange={(e) => set(id, 'date', storedDate(e.currentTarget.value))} />
					</label>
					<label>
						<span class="muted">{m['calendar.title_field']()}</span>
						<input type="text" required value={event.title} oninput={(e) => set(id, 'title', e.currentTarget.value)} />
					</label>
					<label>
						<span class="muted">{m['calendar.city']()}</span>
						<input type="text" required value={event.city} oninput={(e) => set(id, 'city', e.currentTarget.value)} />
					</label>
					<label>
						<span class="muted">{m['calendar.location']()}</span>
						<input type="text" value={event.location} oninput={(e) => set(id, 'location', e.currentTarget.value)} />
					</label>
					<label>
						<span class="muted">{m['calendar.category']()}</span>
						<select value={event.category} onchange={(e) => set(id, 'category', e.currentTarget.value)}>
							{#each categories as c (c.value)}<option value={c.value}>{c.label()}</option>{/each}
						</select>
					</label>
					<label>
						<span class="muted">{m['calendar.link']()}</span>
						<input type="url" value={event.link ?? ''} placeholder="https://" oninput={(e) => set(id, 'link', e.currentTarget.value.trim())} />
					</label>
				</div>
				<button
					type="button"
					class="sc-round"
					onclick={() => (draft = draft.filter((d) => d.id !== id))}
					aria-label={m['calendar.remove']()}
					title={m['calendar.remove']()}
					data-testid="event-remove"><CloseIcon /></button
				>
			</li>
		{/each}
	</ul>
{/snippet}

{#if error}
	<p class="error-box" role="alert">{error}</p>
{:else if !loaded}
	<Skeleton lines={6} label={m['shell.loading']()} />
{:else if draft.length}
	{#if upcoming.length}
		<Section title={m['calendar.upcoming']()}>{@render rows(upcoming)}</Section>
	{/if}
	{#if past.length}
		<Section title={m['calendar.past']()}>{@render rows(past)}</Section>
	{/if}
{/if}

<style>
	.events {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}
	.row {
		display: flex;
		align-items: flex-end;
		gap: var(--space-05, 0.5rem);
		padding: var(--space-05, 0.5rem) 0;
		border-bottom: 1px solid var(--border-subtle, #ddd);
	}
	.fields {
		flex: 1;
		min-width: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
		gap: var(--space-05, 0.5rem);
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}
	input,
	select {
		width: 100%;
		min-width: 0;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
