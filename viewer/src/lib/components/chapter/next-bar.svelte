<script lang="ts">
	import { goto } from '$app/navigation';
	import { isPrologue, Renderer, type Group, type LoadedSequenceItem, type NextStep } from '@scutoons/content';
	import { SLICE_PLAYER_EVENT, SLICE_SKIP_EVENT, type SlicePlayerDetail } from '@scutoons/reader';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { shopUrl } from '$lib/config.ts';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte.ts';
	import { SEEN_THRESHOLDS, sliceSeen } from '$lib/reading/track-reading.ts';
	import { chapterName } from '$lib/utils/chapter-name.ts';
	import { buildJumpToUrl } from '$lib/visual-novel/jump-to.ts';

	/**
	 * The follow-up bar (agents/DESIGN.md "Follow-up bar"): once the shown group – the last of linked groups (a chain,
	 * @scutoons/content chains.ts, all on this page) – is done – its last slice seen, its
	 * player video ended, a VN group's scene finished (no message waiting, no choice open) – a fixed bar at the bottom
	 * says what comes next (`nextStep`): the group's authored end (`Group.end`: offered with the countdown, or with `auto`
	 * gone to at once, no bar), the next group, the next chapter (a 5 s countdown, then it goes there), the
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

	const COUNTDOWN_MS = 5000;

	const vn = getVisualNovelContext();
	const session = getSessionContext();
	const itemsUi = getItemsUiContext();
	const nav = getNavigationContext();

	const count = $derived(slices ?? group.slices?.length ?? 0);
	/** the group ends with a player video: it is done when the video ended (not when it was seen) */
	const endsWithPlayer = $derived(group.slices?.at(-1)?.video?.player === true);
	/**
	 * a full-width strip (renderer.params.fullWidth): it already fills the screen, so the bar sits over its last screen
	 * instead of adding room below it – that room made the page longer right at the strip's end, and the next scroll
	 * pushed the whole strip up (owner 2026-10-06: "the entire page jumps up")
	 */
	const strip = $derived(group.renderer?.type === Renderer.HORIZONTAL && !!group.renderer.params?.fullWidth);

	let host = $state<HTMLElement>();
	let bar = $state<HTMLElement>();
	/** the last slice was read (seen, or its video ended) */
	let reachedEnd = $state(false);
	/** the last slice is (partly) on screen */
	let endInView = $state(false);

	// a VN group: its script ran to the end, nothing waits for a click or a choice; slice triggers (a cucumber offer) too
	const sceneDone = $derived(
		(!group.events || vn.state.eventsDone) && !vn.state.choicesVisible && !(vn.state.messageVisible && vn.state.message.wait)
	);
	const shown = $derived(reachedEnd && sceneDone);

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
		let observed: Element | null = null;
		const io = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					endInView = entry.isIntersecting;
					if (!player && sliceSeen(entry)) reachedEnd = true;
				}
			},
			{ threshold: SEEN_THRESHOLDS }
		);
		const find = () => {
			const last = root.querySelector(`[data-slice-index="${total - 1}"]`);
			if (!last || last === observed) return;
			if (observed) io.unobserve(observed);
			observed = last;
			io.observe(last);
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
	const jumpChapter = $derived(step.kind === 'jump' && step.target.type === 'chapter' && /^\d+$/.test(String(step.target.value)) ? Number(step.target.value) : null);
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
			if (isPrologue(chapter)) return m['start.read']({ season, chapter: step.chapter });
			const name = chapterName(step.chapter);
			return target?.title ? m['next_bar.chapter']({ chapter: name, title: target.title }) : name;
		}
		if (step.kind === 'members') return session.isLoggedIn ? m['next_bar.redeem']() : m['next_bar.sign_in']();
		return m['next_bar.overview']();
	});
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
	const counting = $derived(shown && automatic && !goesAtOnce && !reducedMotion && !cancelled);
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

<div class="next-host" bind:this={host} class:shown={shown && !strip && !goesAtOnce}>
	{#if shown && !goesAtOnce}
		<section
			class="next-bar"
			bind:this={bar}
			aria-label={m['next_bar.label']()}
			data-testid="next-bar"
			data-kind={step.kind}
			data-counting={counting ? 'true' : 'false'}
		>
			<div class="inner">
				{#if message}
					<p class="message" data-testid="next-bar-message">{message}</p>
				{/if}
				<div class="actions">
					<!-- the secondary action (at most one): replay, redeem, where to get a code, the music -->
					{#if automatic && !endsWithPlayer}
						<button type="button" class="secondary" onclick={replay} data-testid="next-bar-replay">
							↺ {m['next_bar.replay']()}
						</button>
					{:else if step.kind === 'members' && !session.isLoggedIn}
						<button type="button" class="secondary" onclick={redeem} data-testid="next-bar-secondary">{m['next_bar.redeem']()}</button>
					{:else if step.kind === 'members'}
						<a class="secondary" href={shopUrl(getLocale())} data-testid="next-bar-secondary">{m['next_bar.where_code']()}</a>
					{:else if step.kind === 'season-end' && download}
						<a class="secondary" href={localizeHref(`/${toonId}/download`)} data-testid="next-bar-secondary">{m['season.download']()}</a>
					{:else}
						<span></span>
					{/if}

					<div class="primary-box">
						{#if primaryHref}
							<a class="primary sc-control" href={primaryHref} onclick={stop} data-testid="next-bar-primary">
								{primaryLabel}
								<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
									<path d="M1 8h13M9 3l5 5-5 5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="square" />
								</svg>
							</a>
						{:else}
							<button type="button" class="primary" onclick={redeem} data-testid="next-bar-primary">
								{primaryLabel}
								<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
									<path d="M1 8h13M9 3l5 5-5 5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="square" />
								</svg>
							</button>
						{/if}
						{#if counting}
							<!-- the countdown: a 2px rule under the button that fills in 5 s -->
							<span class="countdown-rule" aria-hidden="true"><span style:width="{Math.min(100, (elapsed / COUNTDOWN_MS) * 100)}%"></span></span>
							<span class="countdown" data-testid="next-bar-countdown">
								<span aria-live="off">{m['next_bar.countdown']({ seconds: remaining })}</span>
								<button type="button" class="cancel" onclick={stop} data-testid="next-bar-cancel">{m['next_bar.cancel']()}</button>
							</span>
						{/if}
					</div>
				</div>
			</div>
		</section>
	{/if}
</div>

<style>
	/* keeps the last slice clear of the bar once it is shown */
	.next-host.shown {
		height: 9rem;
	}
	.next-bar {
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		/* above the VN box (100; the bar only shows once the scene finished), under the toggles and overlays (1000+) */
		z-index: 110;
		background: var(--paper);
		color: var(--ink);
		border-top: var(--border-width-regular) solid var(--ink);
		font-family: var(--font-mono);
		padding: var(--space-1) var(--space-1) calc(var(--space-1) + env(safe-area-inset-bottom, 0px));
	}
	.inner {
		/* the chapter column's width on phones, a little more where the type is 24 px */
		max-width: max(500px, 30rem);
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
	}
	.message {
		margin: 0;
		font-size: var(--type-body-size);
		line-height: var(--type-body-line);
	}
	.actions {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-1);
	}
	.primary-box {
		flex: none;
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-05);
		min-width: 0;
	}
	.primary {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-075);
		box-sizing: border-box;
		min-height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-15);
		border: 0;
		border-radius: var(--radius-none);
		background: var(--ink);
		color: var(--paper);
		font-family: var(--font-mono);
		font-size: var(--type-button-size);
		line-height: var(--type-button-line);
		text-decoration: none;
		white-space: nowrap;
		cursor: pointer;
		touch-action: manipulation;
	}
	.primary svg {
		flex: none;
		width: 1em;
		height: 1em;
	}
	/* the secondary: a text link in the button type */
	.secondary {
		/* on the primary button's line (same height, one control height), the countdown below it doesn't move it */
		align-self: flex-start;
		display: inline-flex;
		align-items: center;
		box-sizing: border-box;
		min-height: var(--control-height);
		margin: 0;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: var(--type-button-size);
		/* a long one may wrap (the primary never does) */
		line-height: 1.3;
		text-align: left;
		min-width: 0;
		text-decoration: underline;
		text-underline-offset: 0.2em;
		cursor: pointer;
		touch-action: manipulation;
	}
	.countdown-rule {
		display: block;
		height: var(--border-width-regular);
		background: var(--border-muted);
	}
	.countdown-rule span {
		display: block;
		height: 100%;
		background: var(--ink);
	}
	.countdown {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: var(--space-05);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
	}
	.cancel {
		margin: 0;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
</style>
