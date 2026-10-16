<script lang="ts">
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import type { AccountGroup } from '$lib/account/groups.ts';
	import DetailsGroup from './details-group.svelte';
	import ReadingGroup from './reading-group.svelte';
	import FxSetting from './fx-setting.svelte';
	import InstallAppSetting from './install-app-setting.svelte';
	import SavedChaptersSetting from './saved-chapters-setting.svelte';
	import CollectionGroup from './collection-group.svelte';
	import AiConnections from './ai-connections.svelte';
	import CreatorGroup from './creator-group.svelte';
	import AccountToggle from './account-toggle.svelte';

	/** What a group of the account page holds (the order and who sees which: lib/account/groups.ts). */
	let { group }: { group: AccountGroup } = $props();
	const session = getSessionContext();
</script>

{#if group === 'details'}
	<DetailsGroup />
{:else if group === 'reading'}
	<ReadingGroup />
{:else if group === 'sound'}
	<FxSetting />
{:else if group === 'offline'}
	<InstallAppSetting />
	<SavedChaptersSetting />
{:else if group === 'notifications' && session.user}
	<AccountToggle
		checked={!!session.user.marketing_opt_in}
		label={m['account.marketing_opt_in']()}
		hint={m['account.marketing_opt_in_hint']()}
		save={(v) => session.setMarketingOptIn(v)}
		testid="marketing-opt-in"
	/>
{:else if group === 'collection'}
	<CollectionGroup />
{:else if group === 'connections'}
	<p class="hint">{m['account.ai_hint']()}</p>
	<AiConnections />
{:else if group === 'creator'}
	<CreatorGroup />
{/if}

<style>
	.hint {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-muted);
	}
</style>
