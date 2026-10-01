<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Badge, Button, Icon } from '@scutoons/ui';
	import {
		ACTIVITY_KINDS,
		CALL_OUTCOMES,
		STAGES,
		TRACKS,
		type ActivityKind,
		type CallOutcome,
		type Lead,
		type OrgDetail,
		type OrgInput,
		type Person,
		type Stage,
		type TeamMember,
		type Track
	} from '@scutoons/api';
	import { ApiError, embassy } from '$lib/api.ts';
	import { getEmbassySession } from '$lib/session.svelte.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { dateTime, label, relativeDay, shortDate, telHref } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import OrgForm from '$lib/components/org-form.svelte';

	/** One organisation: facts, leads per track, logging a call/mail/note, people, history */
	const session = getEmbassySession();
	const toaster = getToaster();
	const id = $derived(page.params.id ?? '');
	const isAmbassador = $derived(session.hasRole('ambassador') || session.hasRole('admin'));

	let detail = $state<OrgDetail | null>(null);
	let team = $state<TeamMember[]>([]);
	let error = $state<string | null>(null);
	let editing = $state(false);
	let working = $state(false);

	// log form
	let kind = $state<ActivityKind>('call');
	let track = $state<Track | ''>('');
	let personId = $state('');
	let outcome = $state<CallOutcome>('reached');
	let note = $state('');
	let nextAt = $state('');

	// person form
	let personForm = $state<Partial<Person> | null>(null);

	const fail = (e: unknown) => toaster.addToast({ type: 'error', message: e instanceof ApiError ? e.message : String(e) });

	async function load() {
		try {
			detail = await embassy.org(id);
			error = null;
			if (!track && detail.leads[0]) track = detail.leads[0].track;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	$effect(() => {
		void id;
		load();
	});
	$effect(() => {
		embassy.team().then((r) => (team = r.items), () => {});
	});

	async function save(input: OrgInput) {
		working = true;
		try {
			detail = await embassy.updateOrg(id, input);
			editing = false;
			toaster.addToast({ type: 'success', message: m['org.saved']() });
		} catch (e) {
			fail(e);
		} finally {
			working = false;
		}
	}

	async function setLead(lead: Pick<Lead, 'track'>, change: { stage?: Stage; owner_id?: string | null; next_at?: string | null }) {
		try {
			await embassy.setLead(id, lead.track, change);
			await load();
		} catch (e) {
			fail(e);
			await load();
		}
	}

	async function log(event: SubmitEvent) {
		event.preventDefault();
		working = true;
		try {
			const result = await embassy.log(id, {
				kind,
				track: track || null,
				person_id: personId || null,
				outcome: kind === 'call' ? outcome : null,
				note,
				...(nextAt ? { next_at: nextAt } : {})
			});
			note = '';
			nextAt = '';
			const next = result.lead?.next_at;
			toaster.addToast({
				type: 'success',
				message: result.lead ? (next ? m['today.logged']({ date: `${shortDate(next)} (${relativeDay(next)})` }) : m['today.logged_closed']()) : m['org.saved']()
			});
			await load();
		} catch (e) {
			fail(e);
		} finally {
			working = false;
		}
	}

	async function savePerson(event: SubmitEvent) {
		event.preventDefault();
		if (!personForm) return;
		working = true;
		try {
			const { id: personIdToSave, ...fields } = personForm;
			if (personIdToSave) await embassy.updatePerson(personIdToSave, fields);
			else await embassy.addPerson(id, fields);
			personForm = null;
			await load();
		} catch (e) {
			fail(e);
		} finally {
			working = false;
		}
	}

	async function removePerson(person: Person) {
		if (!confirm(m['org.remove_person_confirm']({ name: person.name || person.email }))) return;
		try {
			await embassy.deletePerson(person.id);
			await load();
		} catch (e) {
			fail(e);
		}
	}

	async function remove() {
		if (!detail || !confirm(m['org.delete_confirm']({ name: detail.org.name }))) return;
		try {
			await embassy.deleteOrg(id);
			toaster.addToast({ type: 'success', message: m['org.deleted']() });
			goto('/leads');
		} catch (e) {
			fail(e);
		}
	}

	const missingTracks = $derived(TRACKS.filter((t) => !detail?.leads.some((l) => l.track === t)));
	const personName = (pid: string | null) => detail?.people.find((p) => p.id === pid)?.name ?? '';
	const mayWork = (lead: Lead) => isAmbassador || !lead.owner_id || lead.owner_id === session.user?.id;
	const ownerChoices = (lead: Lead) =>
		isAmbassador ? team : team.filter((t) => t.id === session.user?.id || t.id === lead.owner_id);
</script>

{#if error}
	<p class="error-box">{error}</p>
{:else if !detail}
	<p class="muted">{m['shell.loading']()}</p>
{:else}
	{@const org = detail.org}
	<header class="org-head">
		<h1>{org.name}</h1>
		<div class="row">
			<span class="muted">{label('type', org.type)}{org.city || org.country ? ` · ${[org.city, org.country].filter(Boolean).join(', ')}` : ''}</span>
			{#if org.tier}<Badge tone="accent">{org.tier}</Badge>{/if}
			{#if org.do_not_contact}<Badge tone="error">{m['org.blocked']()}</Badge>{/if}
			{#each org.tags as tag (tag)}<Badge>{tag}</Badge>{/each}
			{#if org.territories.length}<span class="caption muted"><Icon name="map-pin" /> {org.territories.join(' ')}</span>{/if}
		</div>
	</header>

	{#if editing}
		<section class="card">
			<OrgForm {org} onsubmit={save}>
				{#snippet actions()}
					<Button type="submit" disabled={working}>{m['org.save']()}</Button>
					<Button variant="clear" onclick={() => (editing = false)}>{m['org.cancel']()}</Button>
				{/snippet}
			</OrgForm>
		</section>
	{:else}
		<ul class="contacts">
			{#if org.phone}<li><a class="tel" href={telHref(org.phone)}><Icon name="phone" /> {org.phone}</a></li>{/if}
			{#if org.email}<li><a href={`mailto:${org.email}`}><Icon name="mail" /> {org.email}</a></li>{/if}
			{#if org.website}<li><a href={org.website} target="_blank" rel="noopener noreferrer"><Icon name="external-link" /> {org.website.replace(/^https?:\/\//, '')}</a></li>{/if}
			{#each org.links as link (link)}
				<li><a href={link} target="_blank" rel="noopener noreferrer"><Icon name="external-link" /> {link.replace(/^https?:\/\/(www\.)?/, '')}</a></li>
			{/each}
		</ul>
		{#if org.notes}<p class="notes">{org.notes}</p>{/if}
		<p class="caption muted">{m['org.source']()}: {org.source || '–'}</p>
		<Button size="sm" variant="secondary" onclick={() => (editing = true)}>{m['org.edit']()}</Button>
	{/if}

	<h2>{m['org.leads']()}</h2>
	<div class="leads">
		{#each detail.leads as lead (lead.id)}
			<div class="card lead">
				<strong>{label('track', lead.track)}</strong>
				<label class="field">
					<span>{m['org.stage']()}</span>
					<select value={lead.stage} disabled={!mayWork(lead)} onchange={(e) => setLead(lead, { stage: e.currentTarget.value as Stage })}>
						{#each STAGES as s (s)}<option value={s}>{label('stage', s)}</option>{/each}
					</select>
				</label>
				<label class="field">
					<span>{m['org.next_at']()} {lead.next_at ? `· ${relativeDay(lead.next_at)}` : ''}</span>
					<input type="date" value={lead.next_at ?? ''} disabled={!mayWork(lead)} onchange={(e) => setLead(lead, { next_at: e.currentTarget.value || null })} />
				</label>
				<label class="field">
					<span>{m['org.owner']()}</span>
					<select value={lead.owner_id ?? ''} disabled={!mayWork(lead)} onchange={(e) => setLead(lead, { owner_id: e.currentTarget.value || null })}>
						<option value="">{m['org.nobody']()}</option>
						{#each ownerChoices(lead) as member (member.id)}<option value={member.id}>{member.email}</option>{/each}
						{#if lead.owner_id && !ownerChoices(lead).some((t) => t.id === lead.owner_id)}
							<option value={lead.owner_id}>{lead.owner_email}</option>
						{/if}
					</select>
				</label>
			</div>
		{/each}
	</div>
	{#if missingTracks.length}
		<div class="row add-tracks">
			{#each missingTracks as t (t)}
				<Button size="sm" variant="secondary" onclick={() => setLead({ track: t }, {})}>{m['org.add_track']({ track: label('track', t) })}</Button>
			{/each}
		</div>
	{/if}

	<h2>{m['org.log_title']()}</h2>
	<form class="card form-grid" onsubmit={log} data-testid="log-form">
		<label class="field">
			<span>{m['org.kind']()}</span>
			<select bind:value={kind}>
				{#each ACTIVITY_KINDS as k (k)}<option value={k}>{label('kind', k)}</option>{/each}
			</select>
		</label>
		{#if kind === 'call'}
			<label class="field">
				<span>{m['org.outcome']()}</span>
				<select bind:value={outcome}>
					{#each CALL_OUTCOMES as o (o)}<option value={o}>{label('outcome', o).replace(' …', '')}</option>{/each}
				</select>
			</label>
		{/if}
		<label class="field">
			<span>{m['org.track']()}</span>
			<select bind:value={track}>
				<option value="">{m['org.no_track']()}</option>
				{#each TRACKS as t (t)}<option value={t}>{label('track', t)}</option>{/each}
			</select>
		</label>
		<label class="field">
			<span>{m['org.person']()}</span>
			<select bind:value={personId}>
				<option value="">{m['org.nobody_specific']()}</option>
				{#each detail.people as p (p.id)}<option value={p.id}>{p.name || p.email}</option>{/each}
			</select>
		</label>
		<label class="field wide">
			<span>{m['org.note']()}</span>
			<textarea rows="2" bind:value={note}></textarea>
		</label>
		<label class="field">
			<span>{m['org.next_override']()}</span>
			<input type="date" bind:value={nextAt} required={kind === 'call' && outcome === 'call_back'} />
		</label>
		<div class="wide row"><Button type="submit" disabled={working}>{m['org.log']()}</Button></div>
	</form>

	<h2>{m['org.people']()}</h2>
	<ul class="people">
		{#each detail.people as person (person.id)}
			<li class="row">
				<strong>{person.name || '–'}</strong>
				{#if person.role}<span class="muted caption">{person.role}</span>{/if}
				{#if person.phone}<a class="tel" href={telHref(person.phone)}><Icon name="phone" /> {person.phone}</a>{/if}
				{#if person.email}<a href={`mailto:${person.email}`}><Icon name="mail" /> {person.email}</a>{/if}
				{#if person.link}<a href={person.link} target="_blank" rel="noopener noreferrer"><Icon name="external-link" label={person.link} /></a>{/if}
				{#if person.do_not_contact}<Badge tone="error">{m['org.blocked']()}</Badge>{/if}
				<Button size="sm" variant="clear" onclick={() => (personForm = { ...person })}>{m['org.edit']()}</Button>
				{#if isAmbassador}<Button size="sm" variant="clear" onclick={() => removePerson(person)}>{m['org.remove']()}</Button>{/if}
			</li>
		{/each}
	</ul>
	{#if personForm}
		<form class="card form-grid" onsubmit={savePerson}>
			<label class="field"><span>{m['org.person_name']()}</span><input bind:value={personForm.name} /></label>
			<label class="field"><span>{m['org.person_role']()}</span><input bind:value={personForm.role} /></label>
			<label class="field"><span>{m['org.person_phone']()}</span><input type="tel" bind:value={personForm.phone} /></label>
			<label class="field"><span>{m['org.person_email']()}</span><input type="email" bind:value={personForm.email} /></label>
			<label class="field"><span>{m['org.person_link']()}</span><input bind:value={personForm.link} /></label>
			<label class="field"><span>{m['org.person_language']()}</span><input bind:value={personForm.language} maxlength="16" /></label>
			<label class="check wide"><input type="checkbox" bind:checked={personForm.do_not_contact} /> {m['org.do_not_contact']()}</label>
			<div class="wide row">
				<Button type="submit" disabled={working}>{m['org.save']()}</Button>
				<Button variant="clear" onclick={() => (personForm = null)}>{m['org.cancel']()}</Button>
			</div>
		</form>
	{:else}
		<Button size="sm" variant="secondary" onclick={() => (personForm = { name: '', role: '', email: '', phone: '', link: '', language: '' })}>{m['org.add_person']()}</Button>
	{/if}

	<h2>{m['org.timeline']()}</h2>
	{#if !detail.activities.length}
		<p class="muted">{m['org.timeline_empty']()}</p>
	{/if}
	<ol class="timeline">
		{#each detail.activities as a (a.id)}
			<li>
				<span class="caption muted">{dateTime(a.at)} · {a.by_email.split('@')[0]}</span>
				<span>
					<b>{label('kind', a.kind)}{a.outcome ? ` – ${label('outcome', a.outcome).replace(' …', '')}` : ''}</b>
					{#if a.track}<span class="caption"> · {label('track', a.track)}</span>{/if}
					{#if a.person_id}<span class="caption"> · {personName(a.person_id)}</span>{/if}
				</span>
				{#if a.note}<span class="note">{a.note}</span>{/if}
			</li>
		{/each}
	</ol>

	{#if isAmbassador}
		<p class="danger"><Button size="sm" variant="secondary" onclick={remove}>{m['org.delete']()}</Button></p>
	{/if}
{/if}

<style>
	.org-head {
		margin-bottom: var(--space-1);
	}
	.contacts {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-15);
		margin: 0 0 var(--space-075);
		padding: 0;
		list-style: none;
		overflow-wrap: anywhere;
	}
	.tel {
		font-weight: var(--font-weight-bold);
	}
	.notes {
		white-space: pre-line;
		max-width: 44rem;
	}
	.leads {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(15rem, 100%), 1fr));
		gap: var(--space-075);
	}
	.lead {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.add-tracks {
		margin-top: var(--space-075);
	}
	.people {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin: 0 0 var(--space-075);
		padding: 0;
		list-style: none;
		overflow-wrap: anywhere;
	}
	.timeline {
		margin: 0;
		padding: 0;
		list-style: none;
		border-left: var(--border-width-regular) solid var(--ink);
	}
	.timeline li {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		padding: var(--space-05) 0 var(--space-05) var(--space-1);
	}
	.note {
		white-space: pre-line;
	}
	.danger {
		margin-top: var(--space-3);
	}
</style>
