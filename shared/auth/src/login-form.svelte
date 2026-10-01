<script lang="ts" module>
	import type { VerifyResult } from '@scutoons/api';

	/** What the form needs of a session (`createAuthSession`, or an app session built on it) */
	export interface LoginSession {
		requestLogin(
			email: string,
			code?: string | null,
			marketingOptIn?: boolean,
			choice?: { item_id?: string | null; variant?: string | null }
		): Promise<unknown>;
		verifyOtp(email: string, otp: string, marketingOptIn?: boolean): Promise<VerifyResult>;
	}
</script>

<script lang="ts">
	import { Button, Field } from '@scutoons/ui';
	import { ApiError } from '@scutoons/api';
	import { authLabels, type AuthLabels } from './labels.ts';

	/**
	 * Email → one-time code sign-in, the same in every app. The magic link of the mail returns to the app
	 * (`<origin>/?login=<token>`, `session.init(token)`).
	 * `code` = a QR code sent with the request: a membership code is claimed in the same step, a membership or promo code
	 * may create a new account (signup rule). `signup` = the code can create an account: shows the unchecked "Send me news"
	 * box with the one-time code. Apps without signup pass `hint={labels.existingOnly}`.
	 */
	let {
		session,
		locale = 'de',
		labels: overrides = {},
		code = null,
		itemId = null,
		variant = null,
		signup = false,
		hint,
		sentHint,
		onlogin
	}: {
		session: LoginSession;
		locale?: string;
		labels?: Partial<AuthLabels>;
		code?: string | null;
		/** the item/variant picked for a code with options – a new account claims it in the same step */
		itemId?: string | null;
		variant?: string | null;
		signup?: boolean;
		/** below the email field */
		hint?: string;
		/** below the "code sent" line */
		sentHint?: string;
		onlogin?: (result: VerifyResult) => void;
	} = $props();

	const t = $derived(authLabels(locale, overrides));

	let step = $state<'email' | 'otp'>('email');
	let email = $state('');
	let otp = $state('');
	let marketingOptIn = $state(false);
	let working = $state(false);
	let error = $state<string | null>(null);

	function describe(e: unknown) {
		if (e instanceof ApiError && e.status === 0) return t.networkError;
		if (e instanceof ApiError && e.code === 'signup_code_required') return t.signupCodeRequired;
		if (e instanceof ApiError && e.code === 'already_claimed') return t.signupCodeTaken;
		if (e instanceof ApiError && (e.code === 'choose_item' || e.code === 'invalid_item')) return t.chooseItem;
		return t.loginFailed;
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
		const digits = otp.replace(/\D/g, '');
		if (!digits) return;
		working = true;
		error = null;
		try {
			const result = await session.verifyOtp(email, digits, signup ? marketingOptIn : undefined);
			onlogin?.(result);
		} catch (e) {
			error = describe(e);
		} finally {
			working = false;
		}
	}
</script>

<div class="sc-login-form" data-testid="login-form" data-step={step}>
	{#if step === 'email'}
		<form onsubmit={send}>
			<p>{signup ? t.signupIntro : t.loginIntro}</p>
			<Field label={t.email} {hint} {error}>
				{#snippet children({ id, describedby, invalid })}
					<input
						{id}
						type="email"
						name="email"
						autocomplete="email"
						inputmode="email"
						required
						placeholder={t.emailPlaceholder}
						aria-describedby={describedby}
						aria-invalid={invalid}
						bind:value={email}
					/>
				{/snippet}
			</Field>
			<Button type="submit" disabled={working}>{t.sendCode}</Button>
		</form>
	{:else}
		<form onsubmit={verify}>
			<p>{t.codeSent(email)}</p>
			{#if sentHint}
				<p class="hint">{sentHint}</p>
			{/if}
			<Field label={t.otpLabel} {error}>
				{#snippet children({ id, describedby, invalid })}
					<input
						{id}
						type="text"
						name="otp"
						autocomplete="one-time-code"
						inputmode="numeric"
						maxlength={12}
						required
						aria-describedby={describedby}
						aria-invalid={invalid}
						bind:value={otp}
					/>
				{/snippet}
			</Field>
			{#if signup}
				<label class="opt-in">
					<input type="checkbox" name="marketing_opt_in" bind:checked={marketingOptIn} />
					<span>{t.marketingOptIn}</span>
				</label>
			{/if}
			<Button type="submit" disabled={working}>{working ? t.loggingIn : t.verify}</Button>
			<div class="secondary">
				<Button variant="clear" size="sm" onclick={() => ((step = 'email'), (otp = ''), (error = null))}>{t.changeEmail}</Button>
				<Button variant="clear" size="sm" disabled={working} onclick={() => send()}>{t.resend}</Button>
			</div>
		</form>
	{/if}
</div>

<style>
	.sc-login-form form {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-05);
		font-family: var(--font-mono);
	}
	.sc-login-form p {
		margin: 0 0 var(--space-05);
	}
	/* the inputs are styled by @scutoons/ui Field (DESIGN.md form field) */
	.sc-login-form input[name='otp'] {
		letter-spacing: 0.3em;
	}
	.hint {
		font-size: var(--type-caption-size);
		line-height: 1.3;
		color: var(--text-muted);
	}
	.opt-in {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		cursor: pointer;
	}
	.opt-in input {
		flex: none;
		width: 1.25rem;
		height: 1.25rem;
		margin: 0;
		accent-color: var(--ink);
	}
	.secondary {
		display: flex;
		justify-content: space-between;
		gap: var(--space-1);
		font-size: var(--type-caption-size);
	}
</style>
