<script lang="ts">
	import { Badge } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/** Status badges of a code: revoked · expired · claimed / unclaimed · signed */
	let {
		claimed,
		revoked,
		expired = false,
		signed
	}: { claimed: boolean; revoked: boolean; expired?: boolean; signed?: boolean } = $props();
</script>

<span class="badges">
	{#if revoked}<Badge tone="error">{m['codes.status_revoked']()}</Badge>{/if}
	{#if expired}<Badge tone="warning">{m['codes.status_expired']()}</Badge>{/if}
	{#if claimed}<Badge tone="ink">{m['codes.status_claimed']()}</Badge>{:else}<Badge tone="draft">{m['codes.status_unclaimed']()}</Badge>{/if}
	{#if signed}<Badge tone="accent">{m['codes.status_signed']()}</Badge>{/if}
</span>

<style>
	.badges {
		display: inline-flex;
		flex-wrap: wrap;
		gap: var(--space-025);
	}
</style>
