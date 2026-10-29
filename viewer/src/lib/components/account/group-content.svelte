<script lang="ts">
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import type { AccountGroup } from '$lib/account/groups.ts';
	import DetailsGroup from './details-group.svelte';
	import ReadingGroup from './reading-group.svelte';
	import FxSetting from './fx-setting.svelte';
	import InstallAppSetting from './install-app-setting.svelte';
	import AutoSaveSetting from './auto-save-setting.svelte';
	import SavedChaptersSetting from './saved-chapters-setting.svelte';
	import CollectionGroup from './collection-group.svelte';
	import AiConnections from './ai-connections.svelte';
	import CreatorGroup from './creator-group.svelte';
	import DecksGroup from './decks-group.svelte';
	import AccountToggle from './account-toggle.svelte';
	import NotificationsSetting from './notifications-setting.svelte';

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
	<!-- one component per setting (ACCOUNT-PAGE.md §0): the app (install / iOS how-to), save by themselves, the saves -->
	<InstallAppSetting />
	<AutoSaveSetting />
	<SavedChaptersSetting />
{:else if group === 'notifications'}
	<!-- push (this device, also guests – PWA phase 4) and the newsletter (members) -->
	<NotificationsSetting />
	{#if session.isLoggedIn && session.user}
		<AccountToggle
			checked={!!session.user.marketing_opt_in}
			label={m['account.marketing_opt_in']()}
			hint={m['account.marketing_opt_in_hint']()}
			save={(v) => session.setMarketingOptIn(v)}
			testid="marketing-opt-in"
		/>
	{/if}
{:else if group === 'collection'}
	<CollectionGroup />
{:else if group === 'connections'}
	<p class="hint">{m['account.ai_hint']()}</p>
	<AiConnections />
{:else if group === 'decks'}
	<DecksGroup />
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
