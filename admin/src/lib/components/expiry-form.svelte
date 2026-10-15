<script lang="ts">
	import { Button, Field } from '@scutoons/ui';
	import { admin } from '$lib/api.ts';
	import { errorText, minute } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** Change or remove the expiry of one code (server time, like created_at); logged as an `expiry` event by the server. */
	let {
		code,
		expiresAt,
		onchanged
	}: {
		code: string;
		/** qr.expires_at as stored (`Y-m-d H:i:s`), null = never */
		expiresAt: string | null;
		onchanged: (state: { expires_at: string | null; expired: boolean }) => void;
	} = $props();

	const toaster = getToaster();
	/** `Y-m-d H:i:s` → the datetime-local value `Y-m-dTH:i` */
	const local = (value: string | null) => (value ? value.replace(' ', 'T').slice(0, 16) : '');
	let value = $state('');
	let working = $state(false);
	$effect(() => {
		value = local(expiresAt);
	});

	async function save(next: string | null) {
		working = true;
		try {
			const result = await admin.setCodeExpiry(code, next);
			toaster.addToast({
				message: result.expires_at ? m['codes.expiry_saved']({ code, when: minute(result.expires_at) }) : m['codes.expiry_removed']({ code }),
				type: 'success'
			});
			onchanged(result);
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			working = false;
		}
	}

	function submit(event: SubmitEvent) {
		event.preventDefault();
		save(value || null);
	}
</script>

<form class="expiry" onsubmit={submit} data-testid="expiry-form">
	<Field label={m['codes.expires']()} hint={m['codes.expiry_hint']()}>
		{#snippet children({ id, describedby })}
			<input {id} type="datetime-local" aria-describedby={describedby} bind:value />
		{/snippet}
	</Field>
	<div class="buttons">
		{#if expiresAt}<Button size="sm" variant="secondary" disabled={working} onclick={() => save(null)}>{m['codes.remove_expiry']()}</Button>{/if}
		<Button size="sm" type="submit" disabled={working || value === local(expiresAt)}>{m['codes.save_expiry']()}</Button>
	</div>
</form>

<style>
	.expiry {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.buttons {
		display: flex;
		gap: var(--space-05);
	}
</style>
