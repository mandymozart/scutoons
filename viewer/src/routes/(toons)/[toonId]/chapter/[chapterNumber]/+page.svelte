<script lang="ts">
	import MembershipBlocker from '$lib/components/chapter/membership-blocker.svelte';
	import NextBar from '$lib/components/chapter/next-bar.svelte';
	import ChapterTeaser from '$lib/components/chapter/chapter-teaser.svelte';
	import { chainOf, chainSlices, nextStep, seasonMusic } from '@scutoons/content';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import Group from '$lib/components/toon/group.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { audio } from '$lib/actions/audio/index.ts';
	import type { PageProps } from './$types.js';
	import { page } from '$app/state';
	import { afterNavigate } from '$app/navigation';
	import { tick } from 'svelte';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { getGameContext } from '$lib/contexts/game.svelte.ts';
	import { getWorldContext } from '$lib/contexts/world.svelte.ts';
	import { trackReading, type ReadingSpot } from '$lib/reading/track-reading.ts';
	import { chapterName } from '$lib/utils/chapter-name.ts';
	import { showDrafts } from '$lib/draft-preview.svelte.ts';

	let { data }: PageProps = $props();
	// the next-bar's music link: the season's album (related items; older seasons: download.music)
	const inventory = getInventoryContext();
	
	const currentLabel = $derived(page.url.searchParams.get('label') || null);
	
	let group = $derived(
		!data.chapter
			? undefined
			: currentLabel && data.chapter.groups.find((g) => g.label === currentLabel)
			? data.chapter.groups.find((g) => g.label === currentLabel)
			: data.chapter.groups[0]
	);

	// linked groups (owner 2026-10-06, @scutoons/content chains.ts): the shown group's whole chain is one page, its slices
	// numbered on across it; the follow-up bar comes after its last group
	const chain = $derived.by(() => {
		const groups = data.chapter?.groups ?? [];
		const at = group ? groups.indexOf(group) : -1;
		if (at < 0) return null;
		const { start, end } = chainOf(groups, at);
		const { offsets, count } = chainSlices(groups, { start, end });
		return {
			start,
			end,
			at,
			count,
			groups: groups.slice(start, end + 1).map((g, i) => ({ group: g, offset: offsets[i], index: start + i }))
		};
	});
	const lastGroup = $derived(chain ? chain.groups[chain.groups.length - 1].group : undefined);

	// `?label=` of a group further down a chain: the chain from its top, scrolled to that group (instant) – on arrival
	// only (not on "Nochmal", not on back / forward: the browser restores the place)
	afterNavigate(async ({ type }) => {
		if (type === 'popstate') return;
		const c = chain;
		if (!c || c.at === c.start) return;
		const offset = c.groups[c.at - c.start].offset;
		await tick();
		requestAnimationFrame(() => {
			const el = document.querySelector(`.chapter-view [data-slice-index="${offset}"]`);
			if (!el) return;
			// a strip's slice sits in its pinned track: its section's top is the group's top
			const top = (el.closest('.strip') ?? el).getBoundingClientRect().top;
			window.scrollTo({ top: window.scrollY + top, behavior: 'instant' });
		});
	});

	// reading progress for the toon page's start button: how far into the chapter (all its groups' slices), finished at
	// the last slice of its last group (agents/DESIGN.md "Start button")
	const game = getGameContext();
	const world = getWorldContext();
	let visiting = 0;
	const spot = $derived.by((): ReadingSpot | null => {
		const chapter = data.chapter;
		const c = chain;
		if (!chapter || !c) return null;
		const count = (g: (typeof chapter.groups)[number]) => g.slices?.length ?? 0;
		const number = chapter.number;
		// the page = the chain: its slices count from the chain's first group, its end is the chapter's end with the last group
		return {
			offset: chapter.groups.slice(0, c.start).reduce((sum, g) => sum + count(g), 0),
			count: c.count,
			total: chapter.groups.reduce((sum, g) => sum + count(g), 0),
			last: c.end === chapter.groups.length - 1,
			onRead: (position) => {
				const fresh = visiting !== number;
				visiting = number;
				game.readChapter(number, position, fresh);
			},
			// a passed slice unlocks the map places whose unlock is that slice (agents/MAP.md §8.5, universe-wide)
			onSlice: (index) => {
				// the page index → its group's slice
				const owner = c.groups.findLast((g) => g.offset <= index && index < g.offset + count(g.group));
				const image = owner?.group.slices?.[index - owner.offset]?.image;
				if (image && world) world.sliceSeen(page.params.toonId ?? '', number, image);
			}
		};
	});

	// what follows this group (the follow-up bar, agents/DESIGN.md "Follow-up bar")
	const membership = getMembershipContext();
	const step = $derived.by(() => {
		const chapter = data.chapter;
		if (!chapter || !group) return null;
		// the season's whole sequence (drafts too: the next chapter is never skipped), with the loaded list's details
		const listed = data.sequence ?? [];
		const sequence = (data.season?.sequence ?? listed).map((item) => {
			const loaded = listed.find((l) => l.number === Number(item.number));
			return { number: Number(item.number), status: item.status, isPreview: loaded?.isPreview, release: loaded?.release };
		});
		return nextStep({
			groups: chapter.groups,
			group: chapter.groups.indexOf(group),
			chapter: chapter.number,
			sequence,
			includeDrafts: showDrafts(),
			member: membership.isValid
		});
	});
	// "Nochmal": the group (linked groups: from the first one, the page's top) from its start (a VN group runs its script
	// again)
	let replays = $state(0);
	function replay() {
		window.scrollTo({ top: 0 });
		replays++;
	}

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
			{@const gate = data.gate}
			{#snippet text()}
				<p class="gate-badge">{m['chapter.coming_soon']()}</p>
				<h1>{chapterName(gate.number)}{gate.title ? `: ${gate.title}` : ''}</h1>
				{#if gate.release}
					<p class="gate-release">{m['chapter.release']({ date: formatRelease(gate.release) })}</p>
				{/if}
			{/snippet}
			<!-- with a key visual: the fullscreen teaser loop, the text on top (owner 2026-10-05) -->
			{#if gate.keyvisual}
				<ChapterTeaser toonId={page.params.toonId ?? ''} keyvisual={gate.keyvisual} children={text} />
			{:else}
				{@render text()}
			{/if}
		{:else}
			<p>{m['chapter.not_public']()}</p>
		{/if}
	</div>
{:else}
<MembershipBlocker isPreview={data.chapter.isPreview}>
	<div class="chapter-view" use:audio={data.chapter.audio} {@attach trackReading(spot)}>
		<!-- another chain (linked groups: a label inside the same chain keeps the page) or another chapter starts the page
		     from scratch (a VN group runs its events on mount) -->
		{#key `${data.chapter.number}:${chain?.start ?? currentLabel}:${replays}`}
			{#if chain && lastGroup}
				{#each chain.groups as linked (linked.index)}
					<!-- a VN group can only end a chain: further down it starts when its first slice is seen -->
					<Group group={linked.group} sliceOffset={linked.offset} deferScript={linked.index !== chain.start} />
				{/each}
				{#if step}
					<NextBar
						{step}
						group={lastGroup}
						count={chain.count}
						toonId={page.params.toonId ?? ''}
						chapter={data.chapter.number}
						season={data.season?.number ?? 1}
						sequence={data.sequence ?? []}
						download={!!seasonMusic(inventory.catalog, data.season, page.params.toonId ?? '')?.download}
						onreplay={replay}
					/>
				{/if}
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
			margin: 0;
		}
		/* grey on the page; over the teaser it stays white on the scrim */
		> .gate-release {
			color: #6b6b6b;
		}
	}
	.chapter-view {
		max-width: 500px;
		margin: 0 auto;
	}
</style>
