<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ORG_TYPES, TIERS, type Org, type OrgInput } from '@scutoons/api';
	import { label, splitList } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** The organisation's fields; `onsubmit` gets an OrgInput (lists split, country upper case) */
	let {
		org = null,
		onsubmit,
		actions
	}: { org?: Org | null; onsubmit: (input: OrgInput) => void; actions: Snippet } = $props();

	// svelte-ignore state_referenced_locally
	const start = org;
	let name = $state(start?.name ?? '');
	let type = $state<string>(start?.type ?? 'venue');
	let city = $state(start?.city ?? '');
	let country = $state(start?.country ?? '');
	let website = $state(start?.website ?? '');
	let email = $state(start?.email ?? '');
	let phone = $state(start?.phone ?? '');
	let tier = $state<string>(start?.tier ?? '');
	let tags = $state((start?.tags ?? []).join(', '));
	let territories = $state((start?.territories ?? []).join(', '));
	let links = $state((start?.links ?? []).join('\n'));
	let source = $state(start?.source ?? '');
	let notes = $state(start?.notes ?? '');
	let doNotContact = $state(start?.do_not_contact ?? false);

	function submit(event: SubmitEvent) {
		event.preventDefault();
		onsubmit({
			name: name.trim(),
			type: type as Org['type'],
			city: city.trim(),
			country: country.trim().toUpperCase(),
			website: website.trim(),
			email: email.trim(),
			phone: phone.trim(),
			tier: (tier || null) as Org['tier'],
			tags: splitList(tags),
			territories: splitList(territories).map((t) => t.toUpperCase()),
			links: links.split('\n').map((l) => l.trim()).filter(Boolean),
			source: source.trim(),
			notes: notes.trim(),
			do_not_contact: doNotContact
		});
	}
</script>

<form class="form-grid" onsubmit={submit}>
	<label class="field wide">
		<span>{m['org.name']()}</span>
		<input bind:value={name} required maxlength="255" />
	</label>
	<label class="field">
		<span>{m['org.type']()}</span>
		<select bind:value={type}>
			{#each ORG_TYPES as t (t)}<option value={t}>{label('type', t)}</option>{/each}
		</select>
	</label>
	<label class="field">
		<span>{m['org.tier']()}</span>
		<select bind:value={tier}>
			<option value="">{m['org.tier_none']()}</option>
			{#each TIERS as t (t)}<option value={t}>{t}</option>{/each}
		</select>
	</label>
	<label class="field">
		<span>{m['org.city']()}</span>
		<input bind:value={city} />
	</label>
	<label class="field">
		<span>{m['org.country']()}</span>
		<input bind:value={country} maxlength="2" pattern="[A-Za-z]{'{'}2{'}'}" />
	</label>
	<label class="field">
		<span>{m['org.phone']()}</span>
		<input type="tel" bind:value={phone} />
	</label>
	<label class="field">
		<span>{m['org.email']()}</span>
		<input type="email" bind:value={email} />
	</label>
	<label class="field wide">
		<span>{m['org.website']()}</span>
		<input bind:value={website} inputmode="url" />
	</label>
	<label class="field wide">
		<span>{m['org.links']()}</span>
		<textarea rows="2" bind:value={links}></textarea>
	</label>
	<label class="field">
		<span>{m['org.tags']()}</span>
		<input bind:value={tags} />
	</label>
	<label class="field">
		<span>{m['org.territories']()}</span>
		<input bind:value={territories} placeholder="DE, AT, CH" />
	</label>
	<label class="field">
		<span>{m['org.source']()}</span>
		<input bind:value={source} />
	</label>
	<label class="field wide">
		<span>{m['org.notes']()}</span>
		<textarea rows="3" bind:value={notes}></textarea>
	</label>
	<label class="check wide">
		<input type="checkbox" bind:checked={doNotContact} />
		{m['org.do_not_contact']()}
	</label>
	<div class="wide row">{@render actions()}</div>
</form>
