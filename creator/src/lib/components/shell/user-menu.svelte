<script lang="ts">
	import { fade } from 'svelte/transition';
	import { MenuHead, MenuItem, MenuPanel, atBottomEdge, isSwipeUp, motionMs } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';

	/**
	 * The user menu (research/redesign/lap/crafts-v2.html view `user`): it rises from the bottom over everything – the
	 * account, Settings, Switch user (signs out: the sign-in comes back with the address empty) and Sign out. A swipe up
	 * from the bottom edge (touch or pen) or a drag up from the status bar opens it; so does the account icon in the bar.
	 */
	const session = getCreatorSession();
	const who = $derived(session.user?.email ?? '');

	/** a press that may become a swipe up: where it started */
	let press: { id: number; x: number; y: number } | null = null;
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

	function down(event: PointerEvent) {
		if (shell.userMenu || shell.floats?.held || shell.floats?.heldTab) return;
		const target = event.target instanceof Element ? event.target : null;
		const edge = event.pointerType !== 'mouse' && atBottomEdge(event.clientY, window.innerHeight, 1.5 * rem());
		if (edge || target?.closest('.status-bar, [data-swipe-edge]')) press = { id: event.pointerId, x: event.clientX, y: event.clientY };
	}
	function move(event: PointerEvent) {
		if (!press || event.pointerId !== press.id) return;
		if (!isSwipeUp(event.clientX - press.x, event.clientY - press.y, 4 * rem())) return;
		press = null;
		shell.userMenu = true;
	}
	const up = () => (press = null);

	const close = () => (shell.userMenu = false);
	function keydown(event: KeyboardEvent) {
		if (event.key !== 'Escape' || !shell.userMenu) return;
		event.preventDefault();
		event.stopPropagation();
		close();
	}
	const settings = () => (close(), shell.open('settings'));
	const signOut = () => (close(), void session.logout());
</script>

<svelte:window onpointerdowncapture={down} onpointermove={move} onpointerup={up} onpointercancel={up} onkeydowncapture={keydown} />

<!-- the bottom edge: a swipe starts here without the page scrolling -->
<div class="edge" data-swipe-edge aria-hidden="true"></div>

{#if shell.userMenu}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="scrim" onclick={(e) => e.target === e.currentTarget && close()} data-testid="user-menu" transition:fade={{ duration: motionMs('base') }}>
		<div class="sheet" role="dialog" aria-modal="true" aria-label={m['shell.user_menu']()}>
			<MenuPanel width="22rem">
				<MenuHead label={session.roles.join(' · ')} />
				<MenuItem icon="user" label={who} data-testid="user-menu-who" />
				<hr />
				<MenuItem icon="settings" label={m['shell.settings_menu']()} onclick={settings} data-testid="user-menu-settings" />
				<MenuItem icon="users" label={m['shell.switch_user']()} title={m['shell.switch_user_about']()} onclick={signOut} data-testid="switch-user" />
				<MenuItem icon="log-out" label={m['creator.sign_out']()} onclick={signOut} data-testid="sign-out" />
			</MenuPanel>
		</div>
	</div>
{/if}

<style>
	/* along the window's bottom, under the status bar's margin: touch here starts a swipe, never a scroll */
	.edge {
		position: fixed;
		z-index: 46;
		left: 0;
		right: 0;
		bottom: 0;
		height: var(--kit-margin);
		touch-action: none;
	}
	/* over everything – the bar, the panels, a view full screen */
	.scrim {
		position: fixed;
		z-index: 90;
		inset: 0;
		display: flex;
		align-items: flex-end;
		justify-content: center;
		background: color-mix(in srgb, var(--tool-ground) 55%, transparent);
	}
	.sheet {
		margin-bottom: var(--kit-margin);
		animation: rise var(--motion-slow) var(--motion-ease);
	}
	@keyframes rise {
		from {
			transform: translateY(100%);
		}
	}
</style>
