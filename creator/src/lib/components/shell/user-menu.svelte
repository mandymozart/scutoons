<script lang="ts">
	import { Menu, MenuHead, MenuItem } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';

	/**
	 * The user menu, under the account icon in the bar (lap 2 A5): the roles, the account, Settings, Switch user (signs
	 * out: the sign-in comes back with the address empty) and Sign out.
	 */
	const session = getCreatorSession();
	const who = $derived(session?.user?.email ?? '');
	const roles = $derived(session?.roles ?? []);
	const signOut = () => void session?.logout();
</script>

<Menu label={m['shell.account_menu']({ who })} about={m['shell.user_menu']()} icon="user" width="19rem" bind:open={shell.userMenu} testid="bar-account" menuTestid="user-menu">
	{#snippet children(close)}
		{#if roles.length}<MenuHead label={roles.join(' · ')} />{/if}
		<MenuItem icon="user" label={who} data-testid="user-menu-who" />
		<hr />
		<MenuItem icon="settings" label={m['shell.settings']()} onclick={() => (close(), shell.open('settings'))} data-testid="user-menu-settings" />
		<MenuItem icon="users" label={m['shell.switch_user']()} title={m['shell.switch_user_about']()} onclick={() => (close(), signOut())} data-testid="switch-user" />
		<MenuItem icon="log-out" label={m['creator.sign_out']()} onclick={() => (close(), signOut())} data-testid="sign-out" />
	{/snippet}
</Menu>
