<script lang="ts">
	import type { AdminAccountRef } from '@scutoons/api';
	import { m } from '$lib/paraglide/messages.js';

	/** An account in the ownership views: its email linking to the user page; a deleted account shows its id; none = "—" */
	let { account, empty = '—' }: { account: AdminAccountRef | null | undefined; empty?: string } = $props();

	// a line may break before the @ and before each dot of the domain, so tables keep their width
	const parts = $derived.by(() => {
		const email = account?.email ?? '';
		const at = email.lastIndexOf('@');
		return at > 0 ? { local: email.slice(0, at), domain: email.slice(at).split(/(?=\.)/) } : { local: email, domain: [] };
	});
</script>

{#if !account}{empty}{:else if account.email}<a class="account" href="/users/{encodeURIComponent(account.id)}"><span class="local">{parts.local}</span>{#each parts.domain as part, i (i)}<wbr />{part}{/each}</a>{:else}<span class="muted" title={account.id}>{m['claims.deleted_account']()}</span>{/if}

<style>
	.account {
		overflow-wrap: break-word;
	}
	/* a very long name part may break anywhere */
	.local {
		overflow-wrap: anywhere;
	}
</style>
