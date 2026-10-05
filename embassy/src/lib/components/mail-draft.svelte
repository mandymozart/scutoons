<script module lang="ts">
	import type { Track } from '@scutoons/api';

	/** a mail to send: from the Ask agent's draft_mail (tools/mcp embassy.ts) or written on the organisation page */
	export interface Draft {
		id?: string;
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
	// One mail, written by a person or drafted by the Ask agent, sent by the person (owner 2026-10-04: the agent only
	// drafts; any liaison / ambassador sends): one-to-one from the shared mailbox (hello@scutoons.com) through the API,
	// which adds the opt-out line and logs it (mail sent → contacted, call in 6 days). The same card in the Ask panel
	// and on the organisation page. A sent draft is remembered per browser (`id`), so a reopened chat doesn't offer to
	// send it again.
	import { Button, Icon } from '@scutoons/ui';
	import { ApiError, embassy } from '$lib/api.ts';
	import { changed } from '$lib/ask-page.svelte.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { label } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	let {
		draft,
		recipients = [],
		onsent
	}: {
		draft: Draft;
		/** the organisation page: who may get mail (a choice); the Ask card keeps the agent's recipient */
		recipients?: { email: string; person_id: string | null; label: string }[];
		onsent?: () => void;
	} = $props();

	const toaster = getToaster();
	const SENT = 'scutoons_embassy_sent_drafts';
	const sentIds = (): string[] => {
		try {
			return JSON.parse(localStorage.getItem(SENT) ?? '[]');
		} catch {
			return [];
		}
	};

	// the card edits its own copy; the draft it came from stays as the agent wrote it
	// svelte-ignore state_referenced_locally
	let to = $state(draft.to);
	// svelte-ignore state_referenced_locally
	let subject = $state(draft.subject);
	// svelte-ignore state_referenced_locally
	let text = $state(draft.text);
	// svelte-ignore state_referenced_locally
	let done = $state(!!draft.id && sentIds().includes(draft.id));
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
			if (draft.id) {
				try {
					localStorage.setItem(SENT, JSON.stringify([...sentIds(), draft.id].slice(-200)));
				} catch {
					/* storage blocked: the log shows it was sent */
				}
			}
			changed.count++;
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
