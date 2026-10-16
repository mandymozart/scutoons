<script lang="ts">
	// Account page → Benachrichtigungen → "Push" (agents/research/PWA.md §2b, ACCOUNT-PAGE.md §0): one switch – "Tell me
	// when a new chapter is out", on this device (guests too). The browser's permission is asked only by this tap; a
	// refused one puts the switch back and the hint says it is blocked. iPhone / iPad Safari: only the hint that it works
	// in the home-screen app. Nothing at all where push can't work (no worker, no key on the server, the creator's frame).
	import { onMount } from 'svelte';
	import { push } from '$lib/push.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AccountToggle from './account-toggle.svelte';

	onMount(() => void push.init());

	/** the switch: true = it took; false = the browser said no (the hint says why); a failure throws (the error line) */
	async function save(on: boolean): Promise<boolean> {
		await (on ? push.enable() : push.disable());
		if (push.failed) throw new Error('push');
		return (push.state === 'on') === on;
	}
</script>

{#if push.state !== 'hidden'}
	<fieldset class="push-setting" data-testid="notifications-setting" data-state={push.state}>
		<legend>{m['pwa.push_legend']()}</legend>
		{#if push.state === 'install-first'}
			<small>{m['pwa.push_install_first']()}</small>
		{:else}
			<AccountToggle
				checked={push.state === 'on'}
				disabled={push.busy || push.state === 'denied'}
				label={m['pwa.push_new_chapters']()}
				hint={push.state === 'denied' ? m['pwa.push_denied']() : m['pwa.push_hint']()}
				error={m['pwa.push_failed']()}
				{save}
				testid="push-toggle"
			/>
		{/if}
	</fieldset>
{/if}

<style>
	.push-setting {
		border: 0;
		margin: 0;
		padding: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	legend {
		padding: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		font-weight: 700;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
</style>
