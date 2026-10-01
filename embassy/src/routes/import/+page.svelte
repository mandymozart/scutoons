<script lang="ts">
	import { Button } from '@scutoons/ui';
	import type { ImportReport, OrgInput } from '@scutoons/api';
	import { ApiError, embassy } from '$lib/api.ts';
	import { getEmbassySession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** Bulk import (ambassador): the file of scripts/embassy/toplist_import.py, dry run first */
	const session = getEmbassySession();
	const isAmbassador = $derived(session.hasRole('ambassador') || session.hasRole('admin'));

	let organisations = $state<OrgInput[] | null>(null);
	let fileError = $state<string | null>(null);
	let report = $state<ImportReport | null>(null);
	let error = $state<string | null>(null);
	let working = $state(false);

	async function pick(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		organisations = null;
		report = null;
		fileError = null;
		if (!file) return;
		try {
			const data = JSON.parse(await file.text());
			const list = Array.isArray(data) ? data : data?.organisations;
			if (!Array.isArray(list)) throw new Error('no list');
			organisations = list;
		} catch {
			fileError = m['import.invalid']();
		}
	}

	async function run(dryRun: boolean) {
		if (!organisations) return;
		working = true;
		error = null;
		try {
			report = await embassy.import(organisations, dryRun);
		} catch (e) {
			error = e instanceof ApiError ? e.message : String(e);
		} finally {
			working = false;
		}
	}

	const summary = (r: ImportReport) =>
		(r.dry_run ? m['import.report_dry'] : m['import.report'])({ created: r.created, merged: r.merged, people: r.people_added, leads: r.leads_added });
</script>

<h1>{m['import.title']()}</h1>

{#if !isAmbassador}
	<p class="card">{m['import.ambassador_only']()}</p>
{:else}
	<p class="intro">{m['import.intro']()}</p>
	<label class="field file">
		<span>{m['import.file']()}</span>
		<input type="file" accept="application/json,.json" onchange={pick} />
	</label>
	{#if fileError}<p class="error-box">{fileError}</p>{/if}
	{#if organisations}
		<p>{m['import.loaded']({ count: organisations.length })}</p>
		<div class="row">
			<Button variant="secondary" disabled={working} onclick={() => run(true)}>{m['import.dry_run']()}</Button>
			<Button disabled={working || !report?.dry_run} onclick={() => run(false)}>{m['import.run']()}</Button>
		</div>
	{/if}
	{#if error}<p class="error-box">{error}</p>{/if}
	{#if report}
		<div class="card report" data-testid="import-report">
			<p><strong>{summary(report)}</strong></p>
			{#if report.errors.length}
				<p class="muted caption">{m['import.errors']()}</p>
				<ul class="caption">
					{#each report.errors as e (e.index)}<li>#{e.index + 1} {e.name || '–'}: {e.error}</li>{/each}
				</ul>
			{/if}
		</div>
	{/if}
{/if}

<style>
	.intro {
		max-width: 44rem;
	}
	.file {
		max-width: 28rem;
		margin-bottom: var(--space-1);
	}
	.report {
		margin-top: var(--space-1);
	}
</style>
