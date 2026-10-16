<script lang="ts">
	import { page } from '$app/state';
	import { isAccountGroup } from '$lib/account/groups.ts';
	import AccountView from '$lib/components/account/account-view.svelte';

	// one group of the account page (owner 2026-10-16, concept B): /account/reading, /account/sound …
	const group = $derived(isAccountGroup(page.params.group) ? page.params.group : null);
</script>

<AccountView {group} />
