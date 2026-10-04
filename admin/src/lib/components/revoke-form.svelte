<script lang="ts">
	import { Button, confirmDialog, Field } from '@scutoons/ui';
	import { admin } from '$lib/api.ts';
	import { errorText } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** Revoke (with an optional reason) or unrevoke one code; logged as an event by the server. */
	let {
		code,
		revoked,
		reason: currentReason = null,
		formId,
		inlineSubmit = true,
		onchanged
	}: {
		code: string;
		revoked: boolean;
		reason?: string | null;
		/** id of the form, for a submit button elsewhere (the page's ActionBar: `<Button type="submit" form={formId}>`) */
		formId?: string;
		/** false = no own submit button (the ActionBar has it) */
		inlineSubmit?: boolean;
		onchanged: (state: { revoked_at: string | null; revoked_reason: string | null }) => void;
	} = $props();

	const toaster = getToaster();
	let reason = $state('');
	let working = $state(false);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!revoked && !(await confirmDialog({ title: m['codes.revoke_title'](), message: m['codes.revoke_confirm']({ code }), confirmLabel: m['codes.revoke']() }))) return;
		working = true;
		try {
			const result = revoked ? await admin.unrevokeCode(code) : await admin.revokeCode(code, reason.trim());
			reason = '';
			toaster.addToast({ message: revoked ? m['codes.unrevoked']({ code }) : m['codes.revoked']({ code }), type: 'success' });
			onchanged(result);
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			working = false;
		}
	}
</script>

<form class="revoke" id={formId} onsubmit={submit}>
	{#if revoked}
		<p>{m['codes.is_revoked']()}{#if currentReason}: <em>{currentReason}</em>{/if}</p>
		{#if inlineSubmit}<Button type="submit" variant="secondary" size="sm" disabled={working}>{m['codes.unrevoke']()}</Button>{/if}
	{:else}
		<Field label={m['codes.reason']()} hint={m['codes.reason_hint']()}>
			{#snippet children({ id, describedby })}
				<input {id} aria-describedby={describedby} maxlength="255" bind:value={reason} placeholder={m['codes.reason_placeholder']()} />
			{/snippet}
		</Field>
		{#if inlineSubmit}<Button type="submit" size="sm" disabled={working}>{m['codes.revoke']()}</Button>{/if}
	{/if}
</form>

<style>
	.revoke p {
		margin: 0 0 var(--space-05);
	}
</style>
