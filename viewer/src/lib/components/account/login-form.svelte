<script lang="ts">
	import { ApiError } from '$lib/api/client.ts';
	import type { VerifyResult } from '$lib/api/qr.service.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Email → one-time code login. `code` = QR code sent with the request: a membership code is claimed in the same
	 * step, and a membership or promo code allows creating a new account (signup rule, ITEMS.md 2026-09-30).
	 * `signup` = the code can create an account: shows the (unchecked) "Send me news about new releases" checkbox
	 * with the one-time code. Without a code, new addresses get a mail that explains the rule (no one-time code).
	 */
	let {
		code = null,
		itemId = null,
		variant = null,
		signup = false,
		onlogin
	}: {
		code?: string | null;
		/** the item/variant picked for a code with options – a new account claims it in the same step */
		itemId?: string | null;
		variant?: string | null;
		signup?: boolean;
		onlogin?: (result: VerifyResult) => void;
	} = $props();

	const session = getSessionContext();

	let step = $state<'email' | 'otp'>('email');
	let email = $state('');
	let otp = $state('');
	let marketingOptIn = $state(false);
	let working = $state(false);
	let error = $state<string | null>(null);

	function describe(e: unknown) {
		if (e instanceof ApiError && e.status === 0) return m['account.network_error']();
		if (e instanceof ApiError && e.code === 'signup_code_required') return m['account.signup_code_required']();
		if (e instanceof ApiError && e.code === 'already_claimed') return m['account.signup_code_taken']();
		if (e instanceof ApiError && (e.code === 'choose_item' || e.code === 'invalid_item')) return m['claim.choose']();
		return m['account.login_failed']();
	}

	async function send(event?: SubmitEvent) {
		event?.preventDefault();
		if (!email.includes('@')) return;
		working = true;
		error = null;
		try {
			await session.requestLogin(email, code, false, { item_id: itemId, variant });
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
			const result = await session.verifyOtp(email, otp, signup ? marketingOptIn : undefined);
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
			<p>{signup ? m['account.signup_intro']() : m['account.login_intro']()}</p>
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
			{#if !code}
				<p class="hint">{m['account.code_sent_no_account']()}</p>
			{/if}
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
			{#if signup}
				<label class="opt-in">
					<input type="checkbox" name="marketing_opt_in" bind:checked={marketingOptIn} />
					<span>{m['account.marketing_opt_in']()}</span>
				</label>
			{/if}
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
	.hint {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
	.opt-in {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}
	.opt-in input {
		margin: 0;
		width: 1.25rem;
		height: 1.25rem;
		flex: none;
		accent-color: black;
	}
	.opt-in span {
		font-size: 0.875rem;
		color: black;
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
