<script lang="ts" module>
	export type AdminHelpPage =
		| 'dashboard'
		| 'codes'
		| 'code'
		| 'generate'
		| 'batches'
		| 'batch'
		| 'users'
		| 'user'
		| 'events'
		| 'content'
		| 'catalog'
		| 'settings';
</script>

<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The help of every admin page, shown in the ActionBar's Info panel (@scutoons/ui `InfoPanel`, owner 2026-09-30):
	 * definitions and explanations live here; only short field hints and errors stay on the page.
	 */
	let { page }: { page: AdminHelpPage } = $props();
</script>

{#snippet lists()}
	<h3>{m['help.lists_title']()}</h3>
	<p>{m['help.lists']()}</p>
{/snippet}

<div data-testid="page-help-{page}">
	{#if page === 'dashboard'}
		<p>{m['nav.dashboard_hint']()}</p>
		<h3>{m['help.terms_title']()}</h3>
		<ul>
			<li>{m['dashboard.chart_scans_hint']()}</li>
			<li>{m['dashboard.chart_claims_hint']()}</li>
		</ul>
		<h3>{m['dashboard.editions']()}</h3>
		<p>{m['dashboard.editions_hint']()}</p>
	{:else if page === 'codes'}
		<p>{m['help.codes']()}</p>
		<p>{m['codes.section_status_hint']()}</p>
		{@render lists()}
	{:else if page === 'code'}
		<h3>{m['help.code_sections_title']()}</h3>
		<dl>
			<dt>{m['codes.section_facts']()}</dt>
			<dd>{m['codes.section_facts_hint']()}</dd>
			<dt>{m['codes.section_status']()}</dt>
			<dd>{m['codes.section_status_hint']()}</dd>
			<dt>{m['codes.section_activity']()}</dt>
			<dd>{m['codes.section_activity_hint']()}</dd>
			<dt>{m['codes.section_certificate']()}</dt>
			<dd>{m['codes.section_certificate_hint']()}</dd>
			<dt>{m['codes.section_raw']()}</dt>
			<dd>{m['codes.section_raw_hint']()}</dd>
		</dl>
	{:else if page === 'generate'}
		<p>{m['generate.meta']()}</p>
		<h3>{m['generate.how_title']()}</h3>
		<ol>
			<li>{m['generate.how_1']()}</li>
			<li>{m['generate.how_2']()}</li>
			<li>{m['generate.how_3']()}</li>
		</ol>
		<p>{m['generate.how_hint']()}</p>
	{:else if page === 'batches'}
		<p>{m['batches.meta']()}</p>
		<p>{m['help.batches']()}</p>
	{:else if page === 'batch'}
		<p>{m['help.batches']()}</p>
		<h3>{m['batches.manifest']()}</h3>
		<p>{m['batches.manifest_hint']()}</p>
	{:else if page === 'users'}
		<p>{m['help.users']()}</p>
		{@render lists()}
	{:else if page === 'user'}
		<h3>{m['help.code_sections_title']()}</h3>
		<dl>
			<dt>{m['users.section_account']()}</dt>
			<dd>{m['users.section_account_hint']()}</dd>
			<dt>{m['users.section_roles']()}</dt>
			<dd>{m['users.section_roles_hint']()}</dd>
			<dt>{m['users.section_characters']()}</dt>
			<dd>{m['users.section_characters_hint']()}</dd>
			<dt>{m['users.section_codes']()}</dt>
			<dd>{m['users.section_codes_hint']()}</dd>
		</dl>
	{:else if page === 'events'}
		<p>{m['help.events']()}</p>
		{@render lists()}
	{:else if page === 'content'}
		<p>{m['help.content']()}</p>
	{:else if page === 'catalog'}
		<p>{m['catalog.meta']()}</p>
		<h3>{m['catalog.types']()}</h3>
		<p>{m['catalog.types_hint']()}</p>
	{:else if page === 'settings'}
		<p>{m['help.settings']()}</p>
		<h3>{m['settings.section_limits']()}</h3>
		<p>{m['settings.section_limits_hint']()}</p>
		<dl>
			<dt>{m['settings.scan_limit_per_hour']()}</dt>
			<dd>{m['settings.scan_limit_per_hour_hint']()}</dd>
			<dt>{m['settings.lookup_limit_per_hour']()}</dt>
			<dd>{m['settings.lookup_limit_per_hour_hint']()}</dd>
		</dl>
	{/if}
</div>
