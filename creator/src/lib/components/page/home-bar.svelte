<script lang="ts">
	import { openApp } from '@scutoons/pwa';
	import { ActionBar, PhoneBar, type PhoneBarItem } from '@scutoons/ui';
	import { toonName, type ToonIndex } from '@scutoons/content';
	import { VIEWER_URL } from '$lib/config.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { publishTree } from '$lib/editor/publish-tree.ts';
	import { toast } from '$lib/toasts.ts';

	/**
	 * The bar of the top pages (the dashboard, Toons – UI-CONCEPTS E.10, the editors' one bar at every width since
	 * 2026-10-12): Publish everything (BACKLOG 10.60: every draft of every toon and of the world as one list to tick) ·
	 * the reader – icon + a small word. `onpublished` lets the page reload what it shows.
	 */
	let { toons, onpublished }: { toons: ToonIndex[]; onpublished?: () => void } = $props();

	let publishing = $state(false);
	async function publishEverything() {
		publishing = true;
		try {
			const list = toons.map((t) => ({ id: t.id, name: toonName(t, getLocale()) }));
			const result = await publishTree({ kind: 'all', toons: list }, m['creator.publish_tree_title_all']());
			if (result) {
				toast(result.message, { type: result.published.length ? 'success' : 'info' });
				onpublished?.();
			}
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
		} finally {
			publishing = false;
		}
	}
	const items = $derived<PhoneBarItem[]>([
		{ id: 'publish', icon: 'upload-cloud', label: m['creator.publish'](), disabled: publishing || !toons.length, run: publishEverything, testid: 'publish-everything' },
		{ id: 'reader', icon: 'book-open', label: m['creator.reader'](), run: () => openApp(VIEWER_URL), testid: 'open-reader' }
	]);
</script>

<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
	{#snippet phone()}
		<PhoneBar {items} label={m['creator.bar_actions']()} backLabel={m['creator.bar_back']()} />
	{/snippet}
</ActionBar>
