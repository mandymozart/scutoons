<script lang="ts">
	// Account page "Notifications" (agents/research/PWA.md §2b, PWA phase 4): one switch – "Tell me when a new chapter
	// is out". The browser's permission is asked only by this tap. iPhone / iPad Safari: a hint (it works only in the
	// home-screen app). Nothing at all where push can't work (no worker, no key on the server, the creator's frame).
	import { onMount } from 'svelte';
	import { push } from '$lib/push.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	onMount(() => void push.init());

	function toggle(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		// the switch follows the real state, not the tap (a refused permission stays off)
		input.checked = push.state === 'on';
		if (push.state === 'on') void push.disable();
		else void push.enable();
	}
</script>

{#if push.state !== 'hidden'}
	<fieldset class="notifications" data-testid="notifications-setting" data-state={push.state}>
		<legend>{m['pwa.push_title']()}</legend>
		{#if push.state === 'install-first'}
			<small>{m['pwa.push_install_first']()}</small>
		{:else}
			<label class="check">
				<input
					type="checkbox"
					checked={push.state === 'on'}
					disabled={push.busy || push.state === 'denied'}
					onchange={toggle}
					data-testid="push-toggle"
				/>
				<span>
					{m['pwa.push_new_chapters']()}
					<small>
						{push.state === 'denied' ? m['pwa.push_denied']() : push.failed ? m['pwa.push_failed']() : m['pwa.push_hint']()}
					</small>
				</span>
			</label>
		{/if}
	</fieldset>
{/if}

<style>
	.notifications {
		border: 0;
		margin: 0 0 1rem;
		padding: 0;
		min-width: 0;
	}
	/* like the account page's section headings (fx-setting.svelte) */
	legend {
		padding: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		font-weight: 700;
	}
	.check {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		cursor: pointer;
	}
	.check input {
		margin: 0.125rem 0 0;
		width: 1.25rem;
		height: 1.25rem; /* control-ok: a checkbox */
		flex: none;
		accent-color: black;
	}
	.check span {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
</style>
