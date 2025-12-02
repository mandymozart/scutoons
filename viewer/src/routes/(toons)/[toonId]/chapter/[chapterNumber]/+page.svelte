<script lang="ts">
	import MembershipBlocker from '$lib/components/chapter/membership-blocker.svelte';
	import Group from '$lib/components/toon/group.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { audio } from '$lib/actions/audio/index.ts';
	import type { PageProps } from './$types.js';
	import { page } from '$app/state';
	const isDev = import.meta.env.DEV;

	let { data }: PageProps = $props();
	const label = page.url.searchParams.get('label');
	let group = $derived(
		page.url.searchParams.get('label') && data.chapter.groups.find((g) => g.label === label)
			? data.chapter.groups.find((g) => g.label === label)
			: data.chapter.groups[0]
	);
</script>

<MembershipBlocker chapterNumber={data.chapter.number} isPreview={data.chapter.isPreview}>
	<div class="chapter-view" use:audio={data.chapter.audio}>
		{#if isDev}
			<div class="debug">
				<h2>{m.chapter({ number: data.chapter.number })}</h2>
				<p>{data.chapter.description}</p>
				{#if data.chapter.isPreview}
					<p>Preview</p>
				{/if}
			</div>
		{/if}
		{#if group}
			<Group {group} />
		{:else}
			<p>Upsi</p>
		{/if}
	</div>
</MembershipBlocker>

<style>
	.chapter-view {
		max-width: 500px;
		margin: 0 auto;
		.debug {
			position: fixed;
			top: 1rem;
			right: 1rem;
			text-align: right;
			h2 {
				font-size: 0.75rem;
				font-weight: 400;
				margin: 0;
			}
			p {
				font-size: 0.75rem;
				margin: 0;
			}
		}
	}
</style>
