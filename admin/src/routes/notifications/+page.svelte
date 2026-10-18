<script lang="ts">
	// "Benachrichtigungen" (owner 2026-10-18, research/PWA-APPS.md §6): the shared page of every tool – push on this device,
	// the person's kinds, "App installieren". Reached from the signed-in account in the top bar.
	import { NotificationsPage, pwaLabels } from '@scutoons/pwa';
	import { install, push } from '$lib/pwa.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';

	const labels = $derived(pwaLabels(getLocale()));
</script>

<svelte:head><title>{labels.notifications} · SCUTOONS</title></svelte:head>

<div class="notifications-route">
	<h1>{labels.notifications}</h1>
	<NotificationsPage {push} {install} {labels} />
</div>

<style>
	.notifications-route {
		padding: var(--space-1, 1rem);
	}
	h1 {
		margin: 0 0 var(--space-15, 1.5rem);
	}
</style>
