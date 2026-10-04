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
		| 'settings'
		| 'homepage'
		| 'calendar'
		| 'emails'
		| 'assets'
		| 'claims'
		| 'claim';
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
			<dt>{m['claims.section_history']()}</dt>
			<dd>{m['claims.code_history_hint']()}</dd>
			<dt>{m['claims.section_transfer_code']()}</dt>
			<dd>{m['claims.help_transfer_code']()}</dd>
			<dt>{m['codes.section_certificate']()}</dt>
			<dd>{m['codes.section_certificate_hint']()}</dd>
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
	{:else if page === 'calendar'}
		<p>{m['help.calendar']()}</p>
	{:else if page === 'content'}
		<p>{m['help.content']()}</p>
	{:else if page === 'catalog'}
		<p>{m['catalog.meta']()}</p>
		<p>{m['help.catalog']()}</p>
		<h3>{m['help.catalog_history_title']()}</h3>
		<p>{m['help.catalog_history']()}</p>
		<h3>{m['help.catalog_media_title']()}</h3>
		<p>{m['help.catalog_media']()}</p>
		<h3>{m['help.catalog_kinds_title']()}</h3>
		<p>{m['help.catalog_kinds']()}</p>
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
	{:else if page === 'homepage'}
		<p>{m['help.homepage']()}</p>
	{:else if page === 'assets'}
		<p>{m['help.assets']()}</p>
		<h3>{m['help.terms_title']()}</h3>
		<dl>
			<dt>{m['assets.term_asset']()}</dt>
			<dd>{m['assets.term_asset_text']()}</dd>
			<dt>{m['assets.term_version']()}</dt>
			<dd>{m['assets.term_version_text']()}</dd>
			<dt>{m['assets.term_rendition']()}</dt>
			<dd>{m['assets.term_rendition_text']()}</dd>
			<dt>{m['assets.term_usage']()}</dt>
			<dd>{m['assets.term_usage_text']()}</dd>
		</dl>
		{@render lists()}
	{:else if page === 'emails'}
		<p>{m['help.emails']()}</p>
		{@render lists()}
	{:else if page === 'claims'}
		<p>{m['claims.help_list']()}</p>
		<h3>{m['help.terms_title']()}</h3>
		<dl>
			<dt>{m['claims.status_open']()}</dt>
			<dd>{m['claims.help_status_open']()}</dd>
			<dt>{m['claims.status_transferred']()}</dt>
			<dd>{m['claims.help_status_transferred']()}</dd>
			<dt>{m['claims.status_rejected']()}</dt>
			<dd>{m['claims.help_status_rejected']()}</dd>
			<dt>{m['claims.status_withdrawn']()}</dt>
			<dd>{m['claims.help_status_withdrawn']()}</dd>
		</dl>
		{@render lists()}
	{:else if page === 'claim'}
		<p>{m['claims.help_detail']()}</p>
		<h3>{m['claims.help_decide_title']()}</h3>
		<ol>
			<li>{m['claims.help_decide_1']()}</li>
			<li>{m['claims.help_decide_2']()}</li>
			<li>{m['claims.help_decide_3']()}</li>
		</ol>
		<dl>
			<dt>{m['claims.transfer']()}</dt>
			<dd>{m['claims.help_transfer']()}</dd>
			<dt>{m['claims.reject']()}</dt>
			<dd>{m['claims.help_reject']()}</dd>
		</dl>
	{/if}
</div>
