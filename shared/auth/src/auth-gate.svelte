<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button } from '@scutoons/ui';
	import LoginForm from './login-form.svelte';
	import { authLabels, type AuthLabels } from './labels.ts';
	import { canUseApp, type AppId } from './roles.ts';
	import type { AuthSession } from './session.svelte.ts';

	/**
	 * The door of an app with roles (creator, admin, embassy): checking → the sign-in screen → (signed in, but the
	 * account lacks the role) the notice "Your account (x@y) does not have access to this app." with "Switch account"
	 * (= sign out everywhere, then this app's sign-in) → the app (`children`).
	 * With single sign-on an account signed in anywhere arrives here signed in already.
	 */
	let {
		session,
		app,
		appName,
		locale = 'de',
		labels: overrides = {},
		logo = '/SCUTOONS-logo.svg',
		readerHref,
		children
	}: {
		session: AuthSession;
		app: AppId;
		/** shown in the sign-in title, e.g. "Creator" */
		appName: string;
		locale?: string;
		labels?: Partial<AuthLabels>;
		/** logo above the sign-in title (null = none) */
		logo?: string | null;
		/** "Open the reader" on the notice */
		readerHref?: string;
		children: Snippet;
	} = $props();

	const t = $derived(authLabels(locale, overrides));
	const waiting = $derived(!session.ready || (session.loading && !session.me));
	const allowed = $derived(canUseApp(app, session.roles));
	let switching = $state(false);

	async function switchAccount() {
		switching = true;
		try {
			await session.logout();
		} finally {
			switching = false;
		}
	}
</script>

{#if waiting}
	<p class="sc-auth-gate loading" data-testid="{app}-loading">{t.loading}</p>
{:else if !session.isLoggedIn}
	<section class="sc-auth-gate" data-testid="{app}-sign-in">
		{#if logo}<img class="logo" src={logo} alt="SCUTOONS" width="200" />{/if}
		<h1>{t.signInTitle(appName)}</h1>
		<LoginForm {session} {locale} labels={overrides} hint={t.existingOnly} />
	</section>
{:else if !allowed}
	<section class="sc-auth-gate" data-testid="{app}-no-access">
		{#if logo}<img class="logo" src={logo} alt="SCUTOONS" width="200" />{/if}
		<h1>{t.deniedTitle}</h1>
		<p data-testid="auth-denied">{t.denied(session.user?.email ?? '')}</p>
		<p class="actions">
			<Button onclick={switchAccount} disabled={switching} data-testid="auth-switch-account">{t.switchAccount}</Button>
			{#if readerHref}
				<Button variant="secondary" href={readerHref}>{t.openReader}</Button>
			{/if}
		</p>
	</section>
{:else}
	{@render children()}
{/if}

<style>
	.sc-auth-gate {
		box-sizing: border-box;
		max-width: 32rem;
		margin: 0 auto;
		padding: 6rem 1rem 2rem;
		font-family: var(--font-mono);
		font-size: 16px;
		color: var(--ink);
	}
	.sc-auth-gate h1 {
		margin: 0 0 var(--space-1);
		font-size: 1.25rem;
		font-weight: 700;
	}
	.sc-auth-gate p {
		margin: 0 0 var(--space-1);
	}
	.logo {
		display: block;
		margin-bottom: var(--space-1);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
</style>
