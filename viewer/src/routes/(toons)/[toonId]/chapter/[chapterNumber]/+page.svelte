<script lang="ts">
	import MembershipBlocker from '$lib/components/chapter/membership-blocker.svelte';
	import Group from '$lib/components/toon/group.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { audio } from '$lib/actions/audio/index.ts';
	import type { PageProps } from './$types.js';
	import { page } from '$app/state';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { getGameContext } from '$lib/contexts/game.svelte.ts';
	import { trackReading, type ReadingSpot } from '$lib/reading/track-reading.ts';
	import { chapterName } from '$lib/utils/chapter-name.ts';

	let { data }: PageProps = $props();
	
	const currentLabel = $derived(page.url.searchParams.get('label') || null);
	
	let group = $derived(
		!data.chapter
			? undefined
			: currentLabel && data.chapter.groups.find((g) => g.label === currentLabel)
			? data.chapter.groups.find((g) => g.label === currentLabel)
			: data.chapter.groups[0]
	);

	// reading progress for the toon page's start button: how far into the chapter (all its groups' slices), finished at
	// the last slice of its last group (agents/DESIGN.md "Start button")
	const game = getGameContext();
	let visiting = 0;
	const spot = $derived.by((): ReadingSpot | null => {
		const chapter = data.chapter;
		if (!chapter || !group) return null;
		const at = chapter.groups.indexOf(group);
		const count = (g: (typeof chapter.groups)[number]) => g.slices?.length ?? 0;
		const number = chapter.number;
		return {
			offset: chapter.groups.slice(0, at).reduce((sum, g) => sum + count(g), 0),
			count: count(group),
			total: chapter.groups.reduce((sum, g) => sum + count(g), 0),
			last: at === chapter.groups.length - 1,
			onRead: (position) => {
				const fresh = visiting !== number;
				visiting = number;
				game.readChapter(number, position, fresh);
			}
		};
	});

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
			<h1>{chapterName(data.gate.number)}{data.gate.title ? `: ${data.gate.title}` : ''}</h1>
			{#if data.gate.release}
				<p class="gate-release">{m['chapter.release']({ date: formatRelease(data.gate.release) })}</p>
			{/if}
		{:else}
			<p>{m['chapter.not_public']()}</p>
		{/if}
	</div>
{:else}
<MembershipBlocker isPreview={data.chapter.isPreview}>
	<div class="chapter-view" use:audio={data.chapter.audio} {@attach trackReading(spot)}>
		<!-- a new group (label) or another chapter starts the group from scratch (a VN group runs its events on mount) -->
		{#key `${data.chapter.number}:${currentLabel}`}
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
	}
</style>
