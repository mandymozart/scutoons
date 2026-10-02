<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/** The two views of the Emails page (the sent-mail log, the templates): links, the current one inverted. */
	let { current }: { current: 'log' | 'templates' } = $props();

	const views = [
		{ id: 'log', href: '/emails', label: () => m['emails.tab_log']() },
		{ id: 'templates', href: '/emails/templates', label: () => m['emails.tab_templates']() }
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
