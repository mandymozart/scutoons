<script lang="ts">
	import { Button } from '@scutoons/ui';
	import { authLabels, type AuthLabels } from './labels.ts';
	import type { AuthSession } from './session.svelte.ts';

	/**
	 * "Signed in as x@y" + Sign out (= every app, the session is shared). `compact`: one line for a top bar.
	 * Renders nothing while nobody is signed in.
	 */
	let {
		session,
		locale = 'de',
		labels: overrides = {},
		compact = false,
		onlogout
	}: {
		session: Pick<AuthSession, 'user' | 'isLoggedIn' | 'logout'>;
		locale?: string;
		labels?: Partial<AuthLabels>;
		compact?: boolean;
		onlogout?: () => void;
	} = $props();

	const t = $derived(authLabels(locale, overrides));

	async function signOut() {
		await session.logout();
		onlogout?.();
	}
</script>

{#if session.isLoggedIn && session.user}
	<div class="sc-signed-in" class:compact data-testid="signed-in-as">
		<span class="who" title="{t.signedInAs} {session.user.email}"><span class="label">{t.signedInAs}</span> <span class="email">{session.user.email}</span></span>
		<Button variant={compact ? 'clear' : 'secondary'} size={compact ? 'sm' : 'md'} onclick={signOut} data-testid="sign-out">{t.signOut}</Button>
	</div>
{/if}

<style>
	.sc-signed-in {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05) var(--space-1);
		font-family: var(--font-mono);
	}
	.who {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.label {
		color: var(--text-muted);
	}
	.email {
		font-weight: 700; /* bold-ok: the reader's account page; the tools use .compact (regular) */
	}
	.compact {
		flex-wrap: nowrap;
		font-size: var(--type-caption-size);
	}
	.compact .who {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* top bars: the address alone (the label stays for screen readers and in the tooltip) */
	.compact .label {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	.compact .email {
		font-weight: 400;
	}
</style>
