<script lang="ts">
	import { Button, Field } from '@scutoons/ui';
	import { ApiError } from '$lib/api.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** Email → one-time code sign-in for existing accounts (no signup in the creator). */
	const session = getCreatorSession();

	let step = $state<'email' | 'otp'>('email');
	let email = $state('');
	let otp = $state('');
	let working = $state(false);
	let error = $state<string | null>(null);

	const describe = (e: unknown) =>
		e instanceof ApiError && e.status === 0 ? m['login.network_error']() : m['login.login_failed']();

	async function send(event?: SubmitEvent) {
		event?.preventDefault();
		if (!email.includes('@')) return;
		working = true;
		error = null;
		try {
			await session.requestLogin(email);
			step = 'otp';
		} catch (e) {
			error = describe(e);
		} finally {
			working = false;
		}
	}

	async function verify(event?: SubmitEvent) {
		event?.preventDefault();
		if (!otp.trim()) return;
		working = true;
		error = null;
		try {
			await session.verifyOtp(email, otp);
		} catch (e) {
			error = describe(e);
		} finally {
			working = false;
		}
	}
</script>

<div class="login-form">
	{#if step === 'email'}
		<form onsubmit={send}>
			<p>{m['login.login_intro']()}</p>
			<Field label={m['login.email']()} hint={m['login.no_account_hint']()} {error}>
				{#snippet children({ id, describedby, invalid })}
					<input
						{id}
						type="email"
						name="email"
						autocomplete="email"
						inputmode="email"
						required
						placeholder={m['login.email_placeholder']()}
						aria-describedby={describedby}
						aria-invalid={invalid}
						bind:value={email}
					/>
				{/snippet}
			</Field>
			<Button type="submit" disabled={working}>{m['login.send_code']()}</Button>
		</form>
	{:else}
		<form onsubmit={verify}>
			<p>{m['login.code_sent']({ email })}</p>
			<Field label={m['login.otp_label']()} {error}>
				{#snippet children({ id, describedby, invalid })}
					<input
						{id}
						type="text"
						name="otp"
						autocomplete="one-time-code"
						inputmode="numeric"
						required
						aria-describedby={describedby}
						aria-invalid={invalid}
						bind:value={otp}
					/>
				{/snippet}
			</Field>
			<Button type="submit" disabled={working}>
				{working ? m['login.logging_in']() : m['login.verify']()}
			</Button>
			<div class="secondary">
				<Button variant="clear" onclick={() => ((step = 'email'), (otp = ''))}>{m['login.change_email']()}</Button>
				<Button variant="clear" onclick={() => send()}>{m['login.resend']()}</Button>
			</div>
		</form>
	{/if}
</div>

<style>
	form {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-05);
	}
	p {
		margin: 0 0 var(--space-05);
	}
	.secondary {
		display: flex;
		justify-content: space-between;
		gap: var(--space-1);
		font-size: var(--type-caption-size);
	}
</style>
