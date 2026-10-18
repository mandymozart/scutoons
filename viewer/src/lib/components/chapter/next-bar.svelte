<script lang="ts">
	import { goto } from '$app/navigation';
	import { intoOtherToon, isPrologue, Renderer, type Group, type LoadedSequenceItem, type NextStep } from '@scutoons/content';
	import { Icon } from '@scutoons/ui';
	import { SLICE_PLAYER_EVENT, SLICE_SKIP_EVENT, type SlicePlayerDetail } from '@scutoons/reader';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { shopUrl } from '$lib/config.ts';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte.ts';
	import { bandReached, ENDLESS_BAND, SEEN_THRESHOLDS, sliceSeen } from '$lib/reading/track-reading.ts';
	import { chapterName, chapterShownNumber } from '$lib/utils/chapter-name.ts';
	import { buildJumpToUrl } from '$lib/visual-novel/jump-to.ts';
	import { install } from '$lib/install.svelte.ts';

	/**
	 * The chapter's end actions (agents/DESIGN.md "Follow-up bar", BACKLOG 10.48 / UI-CONCEPTS E.5): once the shown group – the last of linked groups (a chain,
	 * @scutoons/content chains.ts, all on this page) – is done – its last slice seen, its
	 * player video ended, a VN group's scene finished (no message waiting, no choice open) – the VN message box (every
	 * page has it, also without a VN) says what comes next, its offers are the box's choices (`nextStep`): the group's authored end (`Group.end`: offered with the countdown, or with `auto`
	 * gone to at once, no box), the next group, the next chapter (a 10 s countdown – a line along the choice's bottom –, then it goes there), the
	 * membership it needs (sign in / redeem a code), coming soon, or the season's end. The countdown stops with
	 * "Abbrechen", when the end scrolls out of view, on any other click (a menu, an overlay) or Escape, and waits while the
	 * tab is hidden; with reduced motion there is none. "Nochmal" replays the group and stops it too (a group ending with a
	 * player video has none: its ↺ is on the video). A player video's "Überspringen" (`sliceskip`) goes to the next slice,
	 * at the group's end to the next group / chapter, else (members, coming soon, the season's end) it shows the bar.
	 */
	let {
		step,
		toonId,
		chapter,
		season,
		sequence,
		group,
		count: slices,
		download = false,
		onreplay
	}: {
		step: NextStep;
		toonId: string;
		/** the shown chapter's number */
		chapter: number;
		/** the season's number ("S1 K1 lesen", "Staffel 1 geschafft") */
		season: number;
		/** the season's listed chapters (titles, release dates) */
		sequence: LoadedSequenceItem[];
		/** the shown group (of linked groups: the last one – its end is the page's end) */
		group: Group;
		/** the page's slices (linked groups: all of theirs, `data-slice-index` 0 … count − 1); default the group's */
		count?: number;
		/** the season has a music download */
		download?: boolean;
		/** start the group (linked groups: the first one) again from its beginning */
		onreplay: () => void;
	} = $props();

	const COUNTDOWN_MS = 10_000; // owner 2026-10-07: 10 s (was 5)

	const vn = getVisualNovelContext();
	// the end actions are choices of the message box (BACKLOG 10.48): the box shows them, the characters stand on it
	$effect(() => {
		if (!shown || goesAtOnce) return;
		return vn.setEnd(endActions);
	});
	/** the page's end reserve: the box's height + its 1 rem above the bottom + 1 rem air (rem, not px) */
	const reserve = $derived.by(() => {
		const px = vn.state.boxHeight;
		if (px <= 0 || typeof document === 'undefined') return '13rem';
		const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
		return `${(px / rem + 2).toFixed(3)}rem`;
	});
	const session = getSessionContext();
	const itemsUi = getItemsUiContext();
	const nav = getNavigationContext();

	const count = $derived(slices ?? group.slices?.length ?? 0);
	/** the group ends with a player video: it is done when the video ended (not when it was seen) */
	const endsWithPlayer = $derived(group.slices?.at(-1)?.video?.player === true);
	/**
	 * the pinned strip (a horizontal group, not endless): it already fills the screen, so the bar sits over its last screen
	 * instead of adding room below it – that room made the page longer right at the strip's end, and the next scroll
	 * pushed the whole strip up (owner 2026-10-06: "the entire page jumps up"). An endless band is in the page flow.
	 */
	const strip = $derived(group.renderer?.type === Renderer.HORIZONTAL && !group.renderer.params?.endless);

	let host = $state<HTMLElement>();
	let bar = $state<HTMLElement>();
	/** the last slice was read (seen, or its video ended) */
	let reachedEnd = $state(false);
	/** the last slice is (partly) on screen */
	let endInView = $state(false);

	// the page's VN is through (owner 2026-10-06): every script it must run finished (linked groups: all from the shown
	// group on), none in line, nothing waits for a click or a choice, no slice trigger running (a cucumber offer)
	const sceneDone = $derived(vn.state.pageDone && !vn.state.frozen);
	const shown = $derived(reachedEnd && sceneDone);
	// a finished chapter (not just a group): the install bar may come on the next page (research/PWA.md "Install prompt")
	$effect(() => {
		if (shown && step.kind !== 'group') install.milestone(`${toonId}/${chapter}`);
	});

	// the end of the group in the chapter view (the bar's host sits in it, after the group)
	$effect(() => {
		const root = host?.closest('.chapter-view');
		if (!root) return;
		const total = count;
		const player = endsWithPlayer;
		if (total <= 0) {
			reachedEnd = true;
			endInView = true;
			return;
		}
		/** the last slice – in an endless band the band too: its last slice sits anywhere in it, maybe never shown */
		let observed: Element[] = [];
		const io = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					const band = entry.target.matches(ENDLESS_BAND);
					// in a band the end is on screen while the band is, and reached once the band's bottom is
					if (band || observed.length === 1) endInView = entry.isIntersecting;
					if (!player && (band ? bandReached(entry) : sliceSeen(entry))) reachedEnd = true;
				}
			},
			{ threshold: SEEN_THRESHOLDS }
		);
		const find = () => {
			const last = root.querySelector(`[data-slice-index="${total - 1}"]`);
			if (!last || last === observed[0]) return;
			for (const el of observed) io.unobserve(el);
			const band = last.closest(ENDLESS_BAND);
			observed = band ? [last, band] : [last];
			for (const el of observed) io.observe(el);
		};
		find();
		// slices rendered later (VN groups, lazy renderers)
		const mo = new MutationObserver(find);
		mo.observe(root, { childList: true, subtree: true });

		const onPlayer = (event: Event) => {
			if (!player) return;
			const detail = (event as CustomEvent<SlicePlayerDetail>).detail;
			const slice = (event.target as HTMLElement | null)?.closest<HTMLElement>('[data-slice-index]');
			if (Number(slice?.dataset.sliceIndex) !== total - 1 || !detail) return;
			if (detail.state === 'ended') reachedEnd = true;
			// played again (its ↺) or moved away from the end (the seek line): the bar goes until it ends again
			else if (detail.state === 'play' || (detail.state === 'time' && detail.progress < 1)) reachedEnd = false;
		};
		const onSkip = (event: Event) => skip(root, event.target as HTMLElement | null);
		root.addEventListener(SLICE_PLAYER_EVENT, onPlayer);
		root.addEventListener(SLICE_SKIP_EVENT, onSkip);
		return () => {
			root.removeEventListener(SLICE_PLAYER_EVENT, onPlayer);
			root.removeEventListener(SLICE_SKIP_EVENT, onSkip);
			mo.disconnect();
			io.disconnect();
		};
	});

	// ── what the bar offers ──────────────────────────────────────────────────────────────────────────────────────────
	/** an authored end to a chapter (`jump`) is named like the next chapter */
	const jumpChapter = $derived(step.kind === 'jump' && step.target.type === 'chapter' && !intoOtherToon(step.target, toonId) && /^\d+$/.test(String(step.target.value)) ? Number(step.target.value) : null);
	const targetChapter = $derived('chapter' in step ? step.chapter : jumpChapter);
	const target = $derived(targetChapter !== null ? sequence.find((item) => item.number === targetChapter) : undefined);
	const chapterHref = (n: number) => localizeHref(`/${toonId}/chapter/${n}`);
	const overviewHref = $derived(localizeHref(`/${toonId}`));
	/** the automatic cases: on to the next group / chapter, or the authored end */
	const automatic = $derived(step.kind === 'group' || step.kind === 'chapter' || step.kind === 'jump');
	/** the authored end's URL (a random one of `anyOf` picked once per showing) */
	const jumpHref = $derived.by(() => {
		if (step.kind !== 'jump') return null;
		const url = buildJumpToUrl(step.target, toonId, group.label ?? null);
		return url ? localizeHref(url) : null;
	});
	const primaryHref = $derived.by(() => {
		if (step.kind === 'jump') return jumpHref;
		if (step.kind === 'group') return localizeHref(`/${toonId}/chapter/${chapter}?label=${encodeURIComponent(step.label)}`);
		if (step.kind === 'chapter') return chapterHref(step.chapter);
		// sign in, then back to the chapter (the account page's `return`)
		if (step.kind === 'members' && !session.isLoggedIn) return localizeHref(`/account?return=${encodeURIComponent(chapterHref(step.chapter))}`);
		if (step.kind === 'members') return null;
		return overviewHref;
	});
	const primaryLabel = $derived.by(() => {
		if (step.kind === 'group') return m['next_bar.next']();
		if (step.kind === 'jump') {
			if (jumpChapter === null || jumpChapter === chapter) return m['next_bar.next']();
			const name = chapterName(jumpChapter);
			return target?.title ? m['next_bar.chapter']({ chapter: name, title: target.title }) : name;
		}
		if (step.kind === 'chapter') {
			// from the prologue like the start button: "S1 K1 lesen"
			if (isPrologue(chapter)) return m['start.read']({ season, chapter: chapterShownNumber(step.chapter) ?? step.chapter });
			const name = chapterName(step.chapter);
			return target?.title ? m['next_bar.chapter']({ chapter: name, title: target.title }) : name;
		}
		if (step.kind === 'members') return session.isLoggedIn ? m['next_bar.redeem']() : m['next_bar.sign_in']();
		return m['next_bar.overview']();
	});
	/** the primary goes on reading (a group, a jump, a chapter): the forward chevron after its label (DESIGN.md "Direction arrows") */
	const forward = $derived(step.kind === 'group' || step.kind === 'jump' || step.kind === 'chapter');
	const message = $derived.by(() => {
		if (step.kind === 'members') {
			const name = chapterName(step.chapter);
			return session.isLoggedIn ? m['next_bar.members_hint_signed_in']({ chapter: name }) : m['next_bar.members_hint']({ chapter: name });
		}
		if (step.kind === 'coming-soon') {
			const name = chapterName(step.chapter);
			return step.release ? m['next_bar.coming_soon']({ chapter: name, date: formatDate(step.release) }) : m['next_bar.coming_soon_undated']({ chapter: name });
		}
		if (step.kind === 'season-end') return m['next_bar.season_end']({ season });
		return '';
	});

	/** YYYY-MM-DD → a date in the reader's language (as written, no time zone shift) */
	function formatDate(release: string) {
		const date = new Date(`${release}T00:00:00Z`);
		if (Number.isNaN(date.getTime())) return release;
		return new Intl.DateTimeFormat(getLocale(), { dateStyle: 'long', timeZone: 'UTC' }).format(date);
	}

	function redeem() {
		stop();
		itemsUi.openCamera({ qr: true, object: null, entry: null });
	}

	function replay() {
		stop();
		onreplay();
	}

	/** "Überspringen" on a player video (not watched): the next slice, else on to what comes next, else the bar */
	function skip(root: Element, target: HTMLElement | null) {
		const index = Number(target?.closest<HTMLElement>('[data-slice-index]')?.dataset.sliceIndex);
		if (!Number.isInteger(index)) return;
		const smooth = reducedMotion ? 'auto' : 'smooth';
		const next = index < count - 1 ? root.querySelector(`[data-slice-index="${index + 1}"]`) : null;
		if (next) {
			next.scrollIntoView({ behavior: smooth, block: 'start' });
			return;
		}
		if (automatic && primaryHref) {
			goto(primaryHref);
			return;
		}
		// members / coming soon / the season's end: the bar says what comes next, the video's end scrolls clear of it
		reachedEnd = true;
		requestAnimationFrame(() => host?.scrollIntoView({ behavior: smooth, block: 'end' }));
	}

	// an authored end with `auto`: there as soon as the group is done – no bar, no countdown
	const goesAtOnce = $derived(step.kind === 'jump' && step.auto && !!primaryHref);
	let went = false;
	$effect(() => {
		if (!shown || !goesAtOnce || went || !primaryHref) return;
		went = true;
		goto(primaryHref);
	});

	// ── the countdown ────────────────────────────────────────────────────────────────────────────────────────────────
	const reducedMotion = typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
	/** stopped for this showing of the bar (the button stays) */
	let cancelled = $state(false);
	let elapsed = $state(0);
	let hidden = $state(typeof document !== 'undefined' && document.visibilityState === 'hidden');
	/** the reader tapped something on this page (outside the bar) since this step came up */
	let tapped = $state(false);
	const counting = $derived(shown && automatic && !goesAtOnce && !reducedMotion && !cancelled && !tapped);
	const remaining = $derived(Math.max(1, Math.ceil((COUNTDOWN_MS - elapsed) / 1000)));

	function stop() {
		cancelled = true;
	}

	// each new showing (another step, shown again after a replay) counts from 0
	const showingKey = $derived(shown ? `${step.kind}:${primaryHref}` : '');
	$effect(() => {
		void showingKey;
		cancelled = false;
		elapsed = 0;
	});

	// the end left the screen (scrolled back up): stop
	$effect(() => {
		if (counting && !endInView) stop();
	});
	// a menu / overlay / the camera opened: stop
	$effect(() => {
		if (counting && (nav.activeOverlay || itemsUi.camera || itemsUi.claimCode)) stop();
	});
	// the reader taps something on the page (picks up an item, taps a figure, answers a choice): not ready to leave –
	// no countdown for this step any more, also when the bar comes back after a figure's talk (owner 2026-10-07: the
	// blotter deal's page went on while the cucumbers were being picked up); the button stays
	const stepKey = $derived(`${step.kind}:${primaryHref}`);
	$effect(() => {
		void stepKey;
		tapped = false;
	});
	$effect(() => {
		const onTap = (e: PointerEvent) => {
			if (!(e.target instanceof Node) || !bar?.contains(e.target)) tapped = true;
		};
		window.addEventListener('pointerdown', onTap, { capture: true });
		return () => window.removeEventListener('pointerdown', onTap, { capture: true });
	});

	$effect(() => {
		if (!counting) return;
		let last = performance.now();
		let frame = 0;
		// a hidden tab waits (rAF doesn't run there; the time away doesn't count)
		const onVisibility = () => {
			hidden = document.visibilityState === 'hidden';
			last = performance.now();
		};
		// any click elsewhere (the main menu, a toggle, the art) or Escape: the reader does something else
		const onPointer = (event: PointerEvent) => {
			if (!bar?.contains(event.target as Node)) stop();
		};
		const onKey = (event: KeyboardEvent) => {
			if (event.key === 'Escape') stop();
		};
		document.addEventListener('visibilitychange', onVisibility);
		document.addEventListener('pointerdown', onPointer, true);
		document.addEventListener('keydown', onKey);
		hidden = document.visibilityState === 'hidden';
		const tick = (now: number) => {
			if (!hidden) elapsed += Math.max(0, now - last);
			last = now;
			if (elapsed >= COUNTDOWN_MS) {
				cancelled = true;
				if (primaryHref) goto(primaryHref);
				return;
			}
			frame = requestAnimationFrame(tick);
		};
		frame = requestAnimationFrame(tick);
		return () => {
			cancelAnimationFrame(frame);
			document.removeEventListener('visibilitychange', onVisibility);
			document.removeEventListener('pointerdown', onPointer, true);
			document.removeEventListener('keydown', onKey);
		};
	});
</script>

<!-- the reserve at the page's end: the last slice can scroll clear of the box (a strip: the box sits over its last screen) -->
<div class="next-host" bind:this={host} class:shown={shown && !strip && !goesAtOnce} style:height={shown && !strip && !goesAtOnce ? reserve : null}></div>

<!-- drawn inside the VN message box (vn/message.svelte) – every page has the box (vn-page.svelte) -->
<!-- a secondary action. Replay and "Abbrechen" are round icon-only buttons at every width (owner 2026-10-17: "Abbrechen
     und replay als ein icon nur"; the word as aria-label / title); the others ("Code einlösen", "Wo bekomme ich einen
     Code?", the music) keep their words -->
{#snippet secondary(tag: 'a' | 'button', label: string, icon: string | null, onclick: (() => void) | null, href: string | null, testid: string, extra = '')}
	{#if tag === 'a'}
		<a class="btn btn-secondary secondary {extra}" class:icon-only={!!icon} href={href} aria-label={label} title={label} data-testid={testid}>
			{#if icon}<Icon name={icon} size="1rem" />{:else}<span class="text">{label}</span>{/if}
		</a>
	{:else}
		<button type="button" class="btn-secondary secondary {extra}" class:icon-only={!!icon} onclick={() => onclick?.()} aria-label={label} title={label} data-testid={testid}>
			{#if icon}<Icon name={icon} size="1rem" />{:else}<span class="text">{label}</span>{/if}
		</button>
	{/if}
{/snippet}

{#snippet endActions()}
	<!-- one compact row (owner 2026-10-17: the three big boxes took too much room – "ein bisschen logischer bauen"; "Passt
	     so … wenn es eine Zeile bleibt"): the secondary (replay / redeem / where to get a code / the music) and, while
	     counting, "Abbrechen" as outlined buttons (replay and Abbrechen round, icon only), the primary last and taking the
	     rest (a long label ends in … only when it doesn't fit); the row never wraps -->
	<section
		class="end-actions"
		bind:this={bar}
		aria-label={m['next_bar.label']()}
		data-testid="next-bar"
		data-kind={step.kind}
		data-counting={counting ? 'true' : 'false'}
	>
		{#if message}
			<p class="message" data-testid="next-bar-message">{message}</p>
		{/if}
		<div class="row">
			{#if automatic && !endsWithPlayer}
				{@render secondary('button', m['next_bar.replay'](), 'rotate-ccw', replay, null, 'next-bar-replay')}
			{:else if step.kind === 'members' && !session.isLoggedIn}
				{@render secondary('button', m['next_bar.redeem'](), null, redeem, null, 'next-bar-secondary')}
			{:else if step.kind === 'members'}
				{@render secondary('a', m['next_bar.where_code'](), null, null, shopUrl(getLocale()), 'next-bar-secondary')}
			{:else if step.kind === 'season-end' && download}
				{@render secondary('a', m['season.download'](), null, null, localizeHref(`/${toonId}/download`), 'next-bar-secondary')}
			{/if}
			{#if counting}
				<span class="countdown" data-testid="next-bar-countdown">
					<span class="sr-only" aria-live="off">{m['next_bar.countdown']({ seconds: remaining })}</span>
					{@render secondary('button', m['next_bar.cancel'](), 'x', stop, null, 'next-bar-cancel', 'cancel')}
				</span>
			{/if}
			{#if primaryHref}
				<a class="btn primary" href={primaryHref} onclick={stop} data-testid="next-bar-primary">
					<span class="label">{primaryLabel}</span>{#if forward}<span class="chevron"><Icon name="chevron-right" size="1rem" /></span>{/if}
					{#if counting}
						<!-- the countdown: a thin line along the primary's bottom that fills in 10 s -->
						<span class="countdown-rule" aria-hidden="true" style:width="{Math.min(100, (elapsed / COUNTDOWN_MS) * 100)}%"></span>
					{/if}
				</a>
			{:else}
				<button type="button" class="primary" onclick={redeem} data-testid="next-bar-primary">
					<span class="label">{primaryLabel}</span>
					{#if counting}
						<span class="countdown-rule" aria-hidden="true" style:width="{Math.min(100, (elapsed / COUNTDOWN_MS) * 100)}%"></span>
					{/if}
				</button>
			{/if}
		</div>
	</section>
{/snippet}

<style>
	.end-actions {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		container-type: inline-size;
	}
	/* the box's text (vn/message.svelte .text) */
	.message {
		margin: 0;
	}
	.row {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: 0.5rem;
	}
	.secondary {
		flex: 0 1 auto;
		min-width: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}
	/* a worded secondary gives way before the primary does: it may end in … on a very narrow phone */
	.secondary .text {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* replay, Abbrechen: round, icon only, the control height */
	.row .icon-only {
		flex: none;
		width: var(--control-height);
		padding: 0;
		border-radius: var(--radius-full);
	}
	/* the primary: last, takes what is left of its line; a long chapter name ends in … */
	.primary {
		position: relative;
		flex: 1 1 auto;
		min-width: 40%;
		max-width: 100%;
		min-width: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.chevron {
		flex: none;
		display: inline-flex;
		margin-left: 0.25rem;
	}
	/* on the black primary: a paper line */
	.countdown-rule {
		position: absolute;
		left: 0;
		bottom: 0;
		height: var(--border-width-regular);
		background: var(--paper);
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border-width: 0;
	}
</style>
