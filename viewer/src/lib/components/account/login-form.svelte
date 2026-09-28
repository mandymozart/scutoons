<script lang="ts">
	import { ApiError } from '$lib/api/client.ts';
	import type { VerifyResult } from '$lib/api/qr.service.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** Email → one-time code login. `code` = QR code to attach to the account in the same step. */
	let {
		code = null,
		onlogin
	}: { code?: string | null; onlogin?: (result: VerifyResult) => void } = $props();

	const session = getSessionContext();

	let step = $state<'email' | 'otp'>('email');
	let email = $state('');
	let otp = $state('');
	let working = $state(false);
	let error = $state<string | null>(null);

	function describe(e: unknown) {
		return e instanceof ApiError && e.status === 0 ? m['account.network_error']() : m['account.login_failed']();
	}

	async function send(event?: SubmitEvent) {
		event?.preventDefault();
		if (!email.includes('@')) return;
		working = true;
		error = null;
		try {
			await session.requestLogin(email, code);
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
			const result = await session.verifyOtp(email, otp);
			onlogin?.(result);
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
			<p>{m['account.login_intro']()}</p>
			<label>
				<span>{m['account.email']()}</span>
				<input
					type="email"
					name="email"
					autocomplete="email"
					inputmode="email"
					required
					placeholder={m['account.email_placeholder']()}
					bind:value={email}
				/>
			</label>
			<button type="submit" class:disabled={working} disabled={working}>{m['account.send_code']()}</button>
		</form>
	{:else}
		<form onsubmit={verify}>
			<p>{m['account.code_sent']({ email })}</p>
			<label>
				<span>{m['account.otp_label']()}</span>
				<input
					type="text"
					name="otp"
					autocomplete="one-time-code"
					inputmode="numeric"
					required
					bind:value={otp}
				/>
			</label>
			<button type="submit" class:disabled={working} disabled={working}>
				{working ? m['account.logging_in']() : m['account.verify']()}
			</button>
			<div class="secondary">
				<button type="button" class="btn-clear" onclick={() => ((step = 'email'), (otp = ''))}>
					{m['account.change_email']()}
				</button>
				<button type="button" class="btn-clear" onclick={() => send()}>{m['account.resend']()}</button>
			</div>
		</form>
	{/if}
	{#if error}
		<p class="error" role="alert">{error}</p>
	{/if}
</div>

<style>
	form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	p {
		margin: 0;
	}
	label span {
		font-size: 0.75rem;
		color: #6b6b6b;
	}
	input {
		margin: 0.25rem 0 0;
	}
	.secondary {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
	}
	.secondary button {
		color: black;
		font-size: 0.75rem;
		text-decoration: underline;
	}
	.error {
		margin-top: 0.5rem;
		border: 2px solid black;
		padding: 0.5rem;
	}
</style>
