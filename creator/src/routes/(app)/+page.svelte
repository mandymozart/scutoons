<script lang="ts">
	import { toonName } from '@scutoons/content';
	import { Button } from '@scutoons/ui/editor';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { notPublished } from '$lib/editor/not-published.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import PagePublish from '$lib/components/shell/page-publish.svelte';

	// nothing open: the stage is empty and the dashboard lies over it (the layout opens it here); the bar's Publish is
	// "publish everything" – nothing is open to publish on its own
	const nav = getCreatorNav();
	const toons = $derived(nav.toons.map((t) => ({ id: t.id, name: toonName(t, getLocale()) })));
	$effect(() => void notPublished.load(toons));
</script>

{#snippet topPublish()}
	<Button variant="primary" icon="upload-cloud" disabled={notPublished.busy || !toons.length} onclick={() => notPublished.publishAll(toons)} data-testid="bar-publish-everything">{m['creator.publish']()} …</Button>
{/snippet}
<PagePublish publish={topPublish} unpublished={notPublished.list.length > 0} />
