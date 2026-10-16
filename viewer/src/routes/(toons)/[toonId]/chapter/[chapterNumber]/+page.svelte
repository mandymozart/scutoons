<script lang="ts">
	import MembershipBlocker from '$lib/components/chapter/membership-blocker.svelte';
	import NextBar from '$lib/components/chapter/next-bar.svelte';
	import ChapterTeaser from '$lib/components/chapter/chapter-teaser.svelte';
	import {
		chainOf,
		chainSlices,
		chapterCheckpointUri,
		checkpointPosition,
		checkpointSpot,
		hasVisualNovel,
		nextStep,
		seasonMusic,
		seasonMusicDownload,
		type ReadingSpotRef
	} from '@scutoons/content';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import Group from '$lib/components/toon/group.svelte';
	import { stripScroll } from '$lib/components/toon/renderer/strip-scroll.ts';
	import VnPage from '$lib/components/toon/interactions/vn-page.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { audio } from '$lib/actions/audio/index.ts';
	import type { PageProps } from './$types.js';
	import { page } from '$app/state';
	import { afterNavigate, goto, replaceState } from '$app/navigation';
	import { onDestroy, tick, untrack } from 'svelte';
	import { creatorLink } from '$lib/creator-link.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { getGameContext } from '$lib/contexts/game.svelte.ts';
	import { trackReading, type ReadingSpot } from '$lib/reading/track-reading.ts';
	import { chapterName } from '$lib/utils/chapter-name.ts';
	import { showDrafts } from '$lib/draft-preview.svelte.ts';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte.ts';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { accessEnd, autoSaveChapter } from '$lib/offline/auto-save.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { isReadable } from '$lib/draft-preview.svelte.ts';
	import { wakeLock } from '$lib/wake-lock.svelte.ts';

	let { data }: PageProps = $props();
	// the next-bar's music link: the season's album (related items; older seasons: download.music)
	const inventory = getInventoryContext();
	
	const currentLabel = $derived(page.url.searchParams.get('label') || null);

	// where the page is: a jump into a group of this chapter that isn't there stays (no dead end – jump-to.ts)
	const vn = getVisualNovelContext();
	// the screen stays on while the reader reads (PWA phase 4, lib/wake-lock.svelte.ts – lapses when idle)
	$effect(() => (data.chapter ? wakeLock.hold('chapter', { idle: true }) : undefined));
	$effect(() => {
		const chapter = data.chapter;
		vn?.setPlace(chapter ? { toon: page.params.toonId ?? '', chapter: chapter.number, labels: chapter.groups.flatMap((g) => (g.label ? [g.label] : [])) } : null);
	});
	
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
	// a VN anywhere on the page (a script or slice events): one box, one script line (vn-page.svelte)
	const pageVn = $derived(!!chain?.groups.some((linked) => hasVisualNovel(linked.group)));

	// `?label=` of a group further down a chain: the chain from its top, scrolled to that group (instant) – on arrival
	// only (not on "Nochmal", not on back / forward: the browser restores the place). On a full page load too: the page
	// may not be there yet when the navigation ends (the membership check, the chain's groups mount) – it waits for the
	// group's first slice (up to WAIT_MS). Kept aligned while the page above settles (a strip measures its sideways
	// travel after it mounted, images load) until the reader scrolls themselves.
	const SETTLE_MS = 2000;
	const WAIT_MS = 10000;
	/** stops the current arrival (a new navigation, the page closing, the reader's own scrolling) */
	let arrival: (() => void) | null = null;
	onDestroy(() => arrival?.());
	afterNavigate(async ({ type, from, to }) => {
		arrival?.();
		arrival = null;
		if (type === 'popstate') return;
		// a jump (a script, a choice, an end) to the very page the reader is on: it starts again, like "Nochmal"
		if (type === 'goto' && from?.url.href === to?.url.href) replays++;
		// "continue" (the start button): the spot the reader stopped at instead (BACKLOG 10.69)
		const chapter = data.chapter;
		if (to?.url.searchParams.has('resume') && chapter) return void resume(chapter.number, chapter.groups);
		const c = chain;
		if (!c || c.at === c.start) return;
		const offset = c.groups[c.at - c.start].offset;
		await tick();
		arrival = arriveAt(offset);
	});
	function arriveAt(offset: number): () => void {
		const target = () => document.querySelector(`.chapter-view [data-slice-index="${offset}"]`);
		const align = () => {
			const el = target();
			if (!el) return;
			// a strip's slice sits in its pinned track: its section's top is the group's top, the page scroll below it moves
			// the track sideways (strip.svelte: the pinned range mapped onto the travel) – a later slice is further down
			const strip = el.closest('.strip');
			const frame = el.closest('.frame');
			const track = frame?.parentElement;
			let top = (strip ?? el).getBoundingClientRect().top;
			if (strip && frame && track) {
				const pin = track.parentElement ?? strip;
				const x = frame.getBoundingClientRect().left - track.getBoundingClientRect().left;
				const travel = Math.max(0, track.scrollWidth - pin.clientWidth);
				top += stripScroll(x, (strip as HTMLElement).offsetHeight - (pin as HTMLElement).offsetHeight, travel);
			}
			if (Math.abs(top) >= 1) window.scrollTo({ top: window.scrollY + top, behavior: 'instant' });
		};
		let observer: ResizeObserver | null = null;
		let frame = 0;
		let timer: ReturnType<typeof setTimeout> | undefined;
		let done = false;
		const events = ['wheel', 'touchstart', 'keydown', 'pointerdown'] as const;
		const stop = () => {
			done = true;
			cancelAnimationFrame(frame);
			observer?.disconnect();
			clearTimeout(timer);
			for (const name of events) window.removeEventListener(name, stop, true);
		};
		for (const name of events) window.addEventListener(name, stop, { capture: true, passive: true });
		const settle = () => {
			align();
			const view = document.querySelector('.chapter-view');
			if (view && typeof ResizeObserver !== 'undefined') {
				observer = new ResizeObserver(align);
				observer.observe(view);
			}
			timer = setTimeout(stop, SETTLE_MS);
		};
		const until = Date.now() + WAIT_MS;
		const wait = () => {
			if (done) return;
			if (target()) return settle();
			if (Date.now() > until) return stop();
			frame = requestAnimationFrame(wait);
		};
		wait();
		return stop;
	}

	// the creator's dock Reader (BACKLOG 10.49, lib/creator-link.svelte.ts): "show group / slice" – a group of the chain
	// on the page is scrolled to; another chain is opened at the group's `?label=` (or the label of a group above it in its
	// chain; the chapter's first chain needs none), then scrolled. Each request once; a group the saved draft hasn't got:
	// nothing happens.
	let shown = 0;
	$effect(() => {
		const r = creatorLink.request;
		const chapter = data.chapter;
		if (!r || r.seq === shown || !chapter || r.toon !== page.params.toonId || r.chapter !== chapter.number) return;
		shown = r.seq;
		untrack(() => void showSpot(r, chapter.groups, () => creatorLink.request?.seq === r.seq));
	});

	// "continue" (the toon page's start button, BACKLOG 10.69): an arrival with `?resume` opens the chapter at the furthest
	// spot of the current read-through (its checkpoint's group / slice); the parameter leaves the URL. A chapter read to
	// its end, or a checkpoint without a spot (from before), opens at its top as before.
	let resumes = 0;
	async function resume(number: number, groups: NonNullable<typeof data.chapter>['groups']) {
		const seq = ++resumes;
		const checkpoint = game.state.progress.find((c) => c.uri === chapterCheckpointUri(page.params.toonId ?? '', number));
		const spot = checkpoint && checkpointPosition(checkpoint) < 1 ? checkpointSpot(checkpoint) : null;
		await tick();
		const url = new URL(location.href);
		url.searchParams.delete('resume');
		try {
			replaceState(url.pathname + url.search + url.hash, page.state);
		} catch {
			// the router not ready yet: the parameter stays, harmless (a reload resumes again)
		}
		if (spot) await showSpot(spot, groups, () => seq === resumes && data.chapter?.number === number);
	}

	/** Scrolls to a group / slice of the chapter, opening its chain first when it is another one; `current` = still wanted */
	async function showSpot(r: ReadingSpotRef, groups: NonNullable<typeof data.chapter>['groups'], current: () => boolean) {
		if (!groups[r.group]) return;
		const inChain = () => !!chain && r.group >= chain.start && r.group <= chain.end;
		if (!inChain()) {
			const { start } = chainOf(groups, r.group);
			const label = groups.slice(start, r.group + 1).findLast((g) => g.label)?.label ?? null;
			if (!label && start !== 0) return;
			const url = new URL(location.href);
			url.searchParams.delete('resume');
			if (label) url.searchParams.set('label', label);
			else url.searchParams.delete('label');
			await goto(url.pathname + url.search, { replaceState: true, noScroll: true, keepFocus: true });
			await tick();
			if (!inChain() || !current()) return;
		}
		const c = chain!;
		arrival?.();
		// the page's first slice: its very top (the header above it too)
		if (r.group === c.start && r.slice <= 0) {
			arrival = null;
			window.scrollTo({ top: 0, behavior: 'instant' });
			return;
		}
		arrival = arriveAt(c.groups[r.group - c.start].offset + Math.max(0, r.slice));
	}

	// reading progress for the toon page's start button: how far into the chapter (all its groups' slices), finished at
	// the last slice of its last group (agents/DESIGN.md "Start button")
	const game = getGameContext();
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
			onRead: (position, index) => {
				const fresh = visiting !== number;
				visiting = number;
				// the spot: the chain's group that slice is in (its index in the chapter) and the slice in that group
				const at = c.groups.findLast((g) => g.offset <= index) ?? c.groups[0];
				game.readChapter(number, position, fresh, { group: at.index, slice: Math.max(0, index - at.offset) });
			}
		};
	});

	// what follows this group (the follow-up bar, agents/DESIGN.md "Follow-up bar")
	const membership = getMembershipContext();
	const session = getSessionContext();
	// "Save chapters on this device" (account setting, lib/offline/auto-save.svelte.ts): a chapter the reader may read is
	// saved by itself once its first screen is there
	$effect(() => {
		const chapter = data.chapter;
		const toon = page.params.toonId ?? '';
		if (!chapter || !toon || !savedChapters.supported) return;
		const listed = data.sequence?.find((item) => item.number === chapter.number);
		if (!isReadable(listed ?? chapter) || !(chapter.isPreview || membership.isValid)) return;
		untrack(() =>
			void autoSaveChapter({
				toon,
				number: chapter.number,
				title: chapter.title ?? '',
				label: chapterName(chapter.number),
				toonTitle: data.toon?.title ?? toon,
				locale: getLocale(),
				accessUntil: accessEnd(membership, session)
			})
		);
	});
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
	// "Nochmal": the group (linked groups: from the first one, the page's top) from its start (every script of the page
	// again, the stage cleared – groups above a `?label=` target stay read)
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

	// the tab title (BACKLOG 10.41): the toon, the chapter's label ("Staffel 2 · Kapitel 1" / "Kapitel 2" / "Prolog"), its title
	const tabTitle = $derived.by(() => {
		const number = data.chapter?.number ?? data.gate?.number ?? Number(page.params.chapterNumber);
		const title = data.chapter?.title || data.gate?.title || data.sequence?.find((item) => item.number === number)?.title || '';
		return [data.toon?.title, `${chapterName(number)}${title ? `: ${title}` : ''}`].filter(Boolean).join(' · ');
	});
</script>

<svelte:head>
	<title>{tabTitle}</title>
</svelte:head>

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
<MembershipBlocker isPreview={data.chapter.isPreview} saved={savedChapters.readable(page.params.toonId ?? '', data.chapter.number)}>
	<!-- data-highlight: how "Show hints" marks the tap areas (the chapter's highlighter – components/toon/fireflies.ts) -->
	<div class="chapter-view" data-highlight={data.chapter.highlight ?? 'fireflies'} use:audio={data.chapter.audio} {@attach trackReading(spot)}>
		<!-- another chain, another group of it (`?label=` – also a jump back into the same chain) or another chapter starts
		     the page from scratch (scripts, stage, the follow-up bar) -->
		{#key `${data.chapter.number}:${chain?.start ?? currentLabel}:${chain?.at ?? ''}:${replays}`}
			{#if chain && lastGroup}
				<VnPage vn={pageVn}>
					<!-- linked groups (owner 2026-10-06): every script starts when its group's first slice is seen, one after
					     another; a lone group's on mount. Groups above the `?label=` target count as read (`done`): no
					     script, no triggers, their text boxes shown -->
					{#each chain.groups as linked (linked.index)}
						<Group
							group={linked.group}
							sliceOffset={linked.offset}
							scope={String(linked.index)}
							deferScript={chain.end > chain.start}
							required={linked.index >= chain.at}
							done={linked.index < chain.at}
						/>
					{/each}
				</VnPage>
				{#if step}
					<NextBar
						{step}
						group={lastGroup}
						count={chain.count}
						toonId={page.params.toonId ?? ''}
						chapter={data.chapter.number}
						season={data.season?.number ?? 1}
						sequence={data.sequence ?? []}
						download={!!seasonMusicDownload(inventory.catalog, seasonMusic(inventory.catalog, data.season, page.params.toonId ?? ''), {
							owned: inventory.entries.map((e) => e.item_id),
							member: membership.isValid
						})}
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
		/* "Coming soon" as a comic stamp (owner 2026-10-06, BACKLOG M.7): the lettering font, a solid ink block, slightly
		   tilted – on the page and over the teaser's picture */
		.gate-badge {
			display: inline-block;
			margin: 0 0 0.5rem;
			padding: 0.125rem 0.625rem 0;
			border: var(--border-width-regular) solid currentColor;
			background: var(--ink);
			color: var(--paper);
			font-family: var(--font-lettering);
			font-size: 1.5rem;
			line-height: 1.25;
			letter-spacing: 0.04em;
			text-transform: uppercase;
			transform: rotate(-3deg);
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
