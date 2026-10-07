<script lang="ts">
	// The first visit of a liaison / ambassador (owner 2026-10-05: "we need an onboarding which lets liaisons set their
	// pace"): what Embassy is for, then their first name (owner 2026-10-07: mails are signed with it), pace and
	// reminder – shown once, until they start; Today has the same choices later.
	import { Button } from '@scutoons/ui';
	import type { PaceReminder } from '@scutoons/api';
	import { ApiError, embassy } from '$lib/api.ts';
	import { saveName, setPace } from '$lib/pace.svelte.ts';
	import { getEmbassySession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PaceChoice from '$lib/components/pace-choice.svelte';
	import WriterName from '$lib/components/writer-name.svelte';

	const session = getEmbassySession();
	let name = $state(session.user?.display_name ?? '');
	let perWeek = $state(10);
	let reminder = $state<PaceReminder>('weekly');
	let saving = $state(false);
	let error = $state('');

	async function start() {
		error = '';
		if (!name.trim()) return void (error = m['name.required']());
		saving = true;
		try {
			await saveName(session, name);
			setPace((await embassy.setPace({ per_week: Math.max(1, Math.min(100, Math.round(perWeek) || 10)), reminder })).pace);
		} catch (e) {
			error = e instanceof ApiError || e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}
</script>

<section class="welcome" data-testid="onboarding">
	<h1>{m['welcome.title']()}</h1>
	<p>{m['welcome.intro']()}</p>
	<ol class="caption">
		<li>{m['welcome.contacts']()}</li>
		<li>{m['welcome.portion']()}</li>
		<li>{m['welcome.pace']()}</li>
	</ol>
	<WriterName bind:value={name} />
	<PaceChoice bind:perWeek bind:reminder />
	{#if error}<p class="error-box">{error}</p>{/if}
	<div><Button onclick={start} disabled={saving}>{m['welcome.start']()}</Button></div>
</section>

<style>
	.welcome {
		max-width: 36rem;
		display: flex;
		flex-direction: column;
		gap: var(--space-15);
	}
	h1,
	p {
		margin: 0;
	}
	ol {
		margin: 0;
		padding-left: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
</style>
