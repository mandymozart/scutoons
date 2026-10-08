<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The views of the Emails page (the sent-mail log, the templates, the newsletters with their contacts and
	 * unsubscribed addresses – agents/NEWSLETTER.md): links, the current one inverted.
	 */
	let { current }: { current: 'log' | 'templates' | 'newsletters' | 'contacts' | 'blocked' } = $props();

	const views = [
		{ id: 'log', href: '/emails', label: () => m['emails.tab_log']() },
		{ id: 'templates', href: '/emails/templates', label: () => m['emails.tab_templates']() },
		{ id: 'newsletters', href: '/emails/newsletters', label: () => m['newsletter.tab_newsletters']() },
		{ id: 'contacts', href: '/emails/contacts', label: () => m['newsletter.tab_contacts']() },
		{ id: 'blocked', href: '/emails/blocked', label: () => m['newsletter.tab_blocked']() }
	] as const;
</script>

<nav class="emails-nav" aria-label={m['emails.tabs']()}>
	{#each views as view (view.id)}
		<a class="sc-nav-item" href={view.href} aria-current={view.id === current ? 'page' : undefined}>{view.label()}</a>
	{/each}
</nav>

<style>
	.emails-nav {
		display: inline-flex;
		flex-wrap: wrap;
		max-width: 100%;
		margin-bottom: var(--space-15);
		border: var(--border-width-regular) solid var(--ink);
	}
	.emails-nav a {
		width: auto;
	}
	.emails-nav a + a {
		border-left: var(--border-width-regular) solid var(--ink);
	}
</style>
