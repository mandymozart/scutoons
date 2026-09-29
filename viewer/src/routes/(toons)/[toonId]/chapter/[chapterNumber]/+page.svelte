<script lang="ts">
	import MembershipBlocker from '$lib/components/chapter/membership-blocker.svelte';
	import Group from '$lib/components/toon/group.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { audio } from '$lib/actions/audio/index.ts';
	import type { PageProps } from './$types.js';
	import { page } from '$app/state';
	import { getLocale } from '$lib/paraglide/runtime.js';
	const isDev = import.meta.env.DEV;

	let { data }: PageProps = $props();
	
	const currentLabel = $derived(page.url.searchParams.get('label') || null);
	
	let group = $derived(
		!data.chapter
			? undefined
			: currentLabel && data.chapter.groups.find((g) => g.label === currentLabel)
			? data.chapter.groups.find((g) => g.label === currentLabel)
			: data.chapter.groups[0]
	);

	/** YYYY-MM-DD → a date in the reader's language (as written, no time zone shift) */
	function formatRelease(release: string) {
		const date = new Date(`${release}T00:00:00Z`);
		if (Number.isNaN(date.getTime())) return release;
		return new Intl.DateTimeFormat(getLocale(), { dateStyle: 'long', timeZone: 'UTC' }).format(date);
	}
</script>

{#if data.gate || !data.chapter}
	<!-- visibility comes from the season status (the load gates drafts in production and coming-soon chapters); the URL stays valid -->
	<div class="chapter-view chapter-gate" data-testid="chapter-gate">
		{#if data.gate?.reason === 'coming-soon'}
			<p class="gate-badge">{m['chapter.coming_soon']()}</p>
			<h1>{m['common.chapter']({ number: data.gate.number })}{data.gate.title ? `: ${data.gate.title}` : ''}</h1>
			{#if data.gate.release}
				<p class="gate-release">{m['chapter.release']({ date: formatRelease(data.gate.release) })}</p>
			{/if}
		{:else}
			<p>{m['chapter.not_public']()}</p>
		{/if}
	</div>
{:else}
<MembershipBlocker isPreview={data.chapter.isPreview}>
	<div class="chapter-view" use:audio={data.chapter.audio}>
		{#if isDev}
			<div class="debug">
				<h2>{m['common.chapter']({ number: data.chapter.number })}</h2>
				<p>{data.chapter.description}</p>
				{#if data.chapter.isPreview}
					<p>Preview</p>
				{/if}
				{#if currentLabel}
					<p>Label: {currentLabel}</p>
				{/if}
			</div>
		{/if}
		{#key currentLabel}
			{#if group}
				<Group {group} />
			{:else}
				<p>Group not found for label: {currentLabel}</p>
			{/if}
		{/key}
	</div>
</MembershipBlocker>
{/if}

<style>
	.chapter-gate {
		padding: 6rem 1rem 2rem;
		text-align: center;
		h1 {
			font-size: 1.25rem;
			margin: 0.5rem 0;
		}
		.gate-badge {
			text-transform: uppercase;
			font-size: 0.75rem;
			margin: 0;
		}
		.gate-release {
			color: #6b6b6b;
			margin: 0;
		}
	}
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
