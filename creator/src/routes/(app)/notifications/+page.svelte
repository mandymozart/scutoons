<script lang="ts">
	// "Benachrichtigungen" (owner 2026-10-18, research/PWA-APPS.md §6): the shared page of every tool – push on this device,
	// the person's kinds, "App installieren" – in the writing page's column (lap 2 C14).
	import { NotificationsPage, pwaLabels } from '@scutoons/pwa';
	import { install, push } from '$lib/pwa.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import WritingPage from '$lib/components/writing/writing-page.svelte';

	const labels = $derived(pwaLabels(getLocale()));
</script>

<svelte:head><title>{labels.notifications} · SCUTOONS</title></svelte:head>

<WritingPage title={labels.notifications} lists testid="notifications-route">
	<NotificationsPage {push} {install} {labels} />
</WritingPage>
