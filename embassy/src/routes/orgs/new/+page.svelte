<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@scutoons/ui';
	import { TRACKS, type OrgInput, type Track } from '@scutoons/api';
	import { ApiError, embassy } from '$lib/api.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { label } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import OrgForm from '$lib/components/org-form.svelte';

	/** New organisation + the tracks to open leads in (the creator owns them) */
	const toaster = getToaster();

	let tracks = $state<Track[]>(['live']);
	let working = $state(false);
	let duplicate = $state<{ id: string; input: OrgInput } | null>(null);
	let error = $state<string | null>(null);

	async function create(input: OrgInput, force = false) {
		working = true;
		error = null;
		try {
			const result = await embassy.createOrg({ ...input, leads: tracks.map((track) => ({ track })), force });
			toaster.addToast({ type: 'success', message: m['org.saved']() });
			goto(`/orgs/${result.org.id}`);
		} catch (e) {
			if (e instanceof ApiError && e.code === 'duplicate') {
				duplicate = { id: String((e.body as Record<string, unknown> | undefined)?.id ?? ''), input };
			} else {
				error = e instanceof Error ? e.message : String(e);
			}
		} finally {
			working = false;
		}
	}
</script>

<h1>{m['org.new_title']()}</h1>

<fieldset class="tracks">
	<legend class="muted caption">{m['org.tracks']()}</legend>
	{#each TRACKS as track (track)}
		<label class="check">
			<input type="checkbox" value={track} bind:group={tracks} />
			{label('track', track)}
		</label>
	{/each}
</fieldset>

{#if duplicate}
	<div class="error-box row dup">
		<span>{m['org.duplicate']()}</span>
		<Button size="sm" href={`/orgs/${duplicate.id}`}>{m['org.open_existing']()}</Button>
		<Button size="sm" variant="secondary" disabled={working} onclick={() => duplicate && create(duplicate.input, true)}>{m['org.create_anyway']()}</Button>
	</div>
{/if}
{#if error}
	<p class="error-box">{error}</p>
{/if}

<OrgForm onsubmit={(input) => create(input)}>
	{#snippet actions()}
		<Button type="submit" disabled={working}>{m['org.create']()}</Button>
	{/snippet}
</OrgForm>

<style>
	.tracks {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		margin: 0 0 var(--space-1);
		padding: 0;
		border: none;
	}
	.tracks legend {
		width: 100%;
		margin-bottom: var(--space-025);
	}
	.dup {
		margin-bottom: var(--space-1);
	}
</style>
