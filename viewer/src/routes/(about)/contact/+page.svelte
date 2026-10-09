<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { ApiError, type ContactMessage } from '@scutoons/api';
	import { sendContact } from '$lib/api.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The contact form (owner 2026-10-09): any request – a QR code that doesn't work, the account, a privacy request,
	 * press / booking. One mail to hello@scutoons.com with the sender as Reply-To (server Domain\Contact\ContactForm);
	 * also the imprint's second contact route next to the email address. `?topic=code&qr=…` opens it prefilled (not `code=` – every page redeems that)
	 * (links from the verify / scan pages). Spam: a hidden honeypot field, the time on the page, a per-IP limit.
	 */
	const TOPICS: ContactMessage['topic'][] = ['general', 'code', 'account', 'privacy', 'press'];
	const topicLabel = (t: ContactMessage['topic']) =>
		({
			general: m['contact.topic_general'](),
			code: m['contact.topic_code'](),
			account: m['contact.topic_account'](),
			privacy: m['contact.topic_privacy'](),
			press: m['contact.topic_press']()
		})[t];

	const session = getSessionContext();
	const asked = page.url.searchParams.get('topic');
	let topic = $state<ContactMessage['topic']>(TOPICS.includes(asked as ContactMessage['topic']) ? (asked as ContactMessage['topic']) : 'general');
	let email = $state(session?.user?.email ?? '');
	let name = $state('');
	let code = $state(page.url.searchParams.get('qr') ?? ''); // not ?code= – the app redeems that on every page
	let message = $state('');
	let website = $state(''); // the honeypot – people never see it
	let opened = 0;
	let sending = $state(false);
	let sent = $state(false);
	let error = $state<string | null>(null);

	onMount(() => (opened = Date.now()));
	// signed in after the page opened: take the account's address while the field is still empty
	$effect(() => {
		if (!email && session?.user?.email) email = session.user.email;
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		sending = true;
		try {
			await sendContact({
				topic,
				email: email.trim(),
				name: name.trim() || undefined,
				code: topic === 'code' && code.trim() ? code.trim() : undefined,
				message,
				locale: getLocale(),
				page: page.url.searchParams.get('from') ?? undefined,
				website,
				elapsed: opened ? Date.now() - opened : 0
			});
			sent = true;
		} catch (e) {
			const code = e instanceof ApiError ? e.code : '';
			error =
				code === 'invalid_email'
					? m['contact.error_email']()
					: code === 'message_required'
						? m['contact.error_message']()
						: e instanceof ApiError && e.status === 429
							? m['contact.error_limit']()
							: m['contact.error_failed']();
		} finally {
			sending = false;
		}
	}
</script>

<svelte:head><title>{m['contact.title']()} · SCUTOONS</title></svelte:head>

<div class="page" data-testid="contact">
	<h1>{m['contact.title']()}</h1>
	{#if sent}
		<p class="done" role="status" data-testid="contact-sent">{m['contact.sent']()}</p>
	{:else}
		<p class="intro">{m['contact.intro']()}</p>
		<form onsubmit={submit} novalidate>
			<label>
				<span>{m['contact.topic']()}</span>
				<select bind:value={topic} data-testid="contact-topic">
					{#each TOPICS as t (t)}<option value={t}>{topicLabel(t)}</option>{/each}
				</select>
			</label>
			{#if topic === 'code'}
				<label>
					<span>{m['contact.code']()}</span>
					<input bind:value={code} maxlength="64" autocomplete="off" placeholder={m['contact.code_placeholder']()} />
				</label>
			{/if}
			<label>
				<span>{m['contact.email']()}</span>
				<input type="email" bind:value={email} required maxlength="255" autocomplete="email" data-testid="contact-email" />
			</label>
			<label>
				<span>{m['contact.name']()}</span>
				<input bind:value={name} maxlength="100" autocomplete="name" />
			</label>
			<label>
				<span>{m['contact.message']()}</span>
				<textarea bind:value={message} required rows="8" maxlength="5000" data-testid="contact-message"></textarea>
			</label>
			<!-- honeypot: off screen, not reachable by keyboard, ignored by autofill – bots fill it in -->
			<label class="trap" aria-hidden="true">
				Website <input tabindex="-1" autocomplete="off" bind:value={website} />
			</label>
			{#if error}<p class="error" role="alert">{error}</p>{/if}
			<p class="note">{m['contact.note']()}</p>
			<button type="submit" disabled={sending} data-testid="contact-send">{sending ? m['contact.sending']() : m['contact.send']()}</button>
		</form>
	{/if}
	<p class="mail">{m['contact.or_mail']()} <a href="mailto:hello@scutoons.com">hello@scutoons.com</a></p>
</div>

<style>
	.page {
		box-sizing: border-box;
		max-width: 40rem;
		margin: 0 auto;
		padding: 5rem 1rem 3rem;
	}
	label {
		display: block;
	}
	label span {
		display: block;
		margin-top: 1rem;
		font-size: 0.875rem;
	}
	label input,
	label select,
	label textarea {
		margin: 0.25rem 0 0;
	}
	.trap {
		position: absolute;
		left: -100rem;
		width: 1px;
		height: 1px;
		overflow: hidden;
	}
	.note,
	.mail {
		font-size: 0.875rem;
		color: var(--text-muted);
	}
	.note {
		margin: 1rem 0;
	}
	.mail {
		margin-top: 2rem;
	}
	.error {
		color: var(--danger, #c0392b);
	}
	.done {
		font-size: 1.125rem;
	}
</style>
