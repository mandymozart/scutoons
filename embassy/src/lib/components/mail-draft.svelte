<script module lang="ts">
	import type { Track } from '@scutoons/api';

	/** a mail to send, written on the organisation page */
	export interface Draft {
		org_id: string;
		org?: string;
		to: string;
		name?: string;
		person_id?: string | null;
		track?: Track | null;
		subject: string;
		text: string;
		reply_to_activity_id?: string | null;
	}
</script>

<script lang="ts">
	// One mail, written and sent by a person (any liaison / ambassador): one-to-one from the shared mailbox
	// (hello@scutoons.com) through the API, which adds the opt-out line and logs it (mail sent → contacted, call in 6
	// days). On the organisation page.
	import { Button, Icon } from '@scutoons/ui';
	import { ApiError, embassy } from '$lib/api.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { label } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	let {
		draft,
		recipients = [],
		onsent
	}: {
		draft: Draft;
		/** who may get mail (a choice); without, the draft's recipient */
		recipients?: { email: string; person_id: string | null; label: string }[];
		onsent?: () => void;
	} = $props();

	const toaster = getToaster();
	// the card edits its own copy of the draft
	// svelte-ignore state_referenced_locally
	let to = $state(draft.to);
	// svelte-ignore state_referenced_locally
	let subject = $state(draft.subject);
	// svelte-ignore state_referenced_locally
	let text = $state(draft.text);
	let done = $state(false);
	let sending = $state(false);
	let error = $state('');

	async function send(event: SubmitEvent) {
		event.preventDefault();
		if (!subject.trim() || !text.trim()) return void (error = m['mail.empty']());
		sending = true;
		error = '';
		try {
			const chosen = recipients.find((r) => r.email === to);
			await embassy.sendMail(draft.org_id, {
				to,
				person_id: chosen ? chosen.person_id : (draft.person_id ?? null),
				track: draft.track ?? null,
				subject: subject.trim(),
				text,
				reply_to_activity_id: draft.reply_to_activity_id ?? null
			});
			done = true;
			toaster.addToast({ type: 'success', message: m['mail.sent']() });
			onsent?.();
		} catch (e) {
			error = e instanceof ApiError && e.code === 'do_not_contact' ? m['mail.blocked']() : e instanceof Error ? e.message : String(e);
		} finally {
			sending = false;
		}
	}
</script>

<form class="mail" onsubmit={send} data-testid="mail-draft">
	<p class="head">
		<Icon name="mail" size="1em" />
		<span>{draft.reply_to_activity_id ? m['mail.reply']() : m['mail.draft']()}{draft.org ? ` · ${draft.org}` : ''}{draft.track ? ` · ${label('track', draft.track)}` : ''}</span>
	</p>
	<label class="field">
		<span>{m['mail.to']()}</span>
		{#if recipients.length > 1}
			<select bind:value={to} disabled={done}>
				{#each recipients as r (r.email)}<option value={r.email}>{r.label}</option>{/each}
			</select>
		{:else}
			<input value={draft.name && draft.name !== to ? `${draft.name} <${to}>` : to} readonly />
		{/if}
	</label>
	<label class="field"><span>{m['mail.subject']()}</span><input bind:value={subject} maxlength="300" readonly={done} /></label>
	<label class="field"><span>{m['mail.text']()}</span><textarea bind:value={text} rows="8" readonly={done}></textarea></label>
	{#if error}<p class="sc-error">{error}</p>{/if}
	<div class="row">
		{#if done}
			<span class="caption"><Icon name="check" size="1em" /> {m['mail.sent']()}</span>
		{:else}
			<span class="caption muted">{m['mail.footer_note']()}</span>
			<Button type="submit" disabled={sending}>{sending ? m['mail.sending']() : m['mail.send']()}</Button>
		{/if}
	</div>
</form>

<style>
	.mail {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin-top: var(--space-1);
		padding: var(--space-1);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin: 0;
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.field > span {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	textarea {
		resize: vertical;
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
	}
	.caption {
		font-size: var(--type-caption-size);
	}
	.muted {
		color: var(--text-muted);
	}
</style>
