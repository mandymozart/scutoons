<script lang="ts">
	import { fromAction } from 'svelte/attachments';
	import { hidesSensitive, isSensitive, pickQualityVariant, sanitizeNumber, type Dialog, type Slice as SliceModel, type SliceItem } from '@scutoons/content';
	import { Slice, sliceAudio, type DialogPlay, type PlayerLabels, type ResolveSliceItem } from '@scutoons/reader';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { getCurrentInventory } from '$lib/contexts/inventory.svelte.ts';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { isTaken, pickUpSliceItem, runSliceItemEvents } from '$lib/visual-novel/slice-items.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { audioPlayer } from '$lib/actions/audio/index.ts';
	import { mediaQuality } from '$lib/media-quality.svelte.ts';
	import Samples from './audio/samples.svelte';
	import Triggers from './vn/triggers.svelte';

	let {
		slice,
		sliceIndex = 0,
		pageIndex,
		allSlices = []
	}: {
		slice: SliceModel;
		/** the slice's index in its group (`allSlices`): the music state, eager loading */
		sliceIndex?: number;
		/**
		 * its index on the page (`data-slice-index`: reading progress, the follow-up bar, a player's skip): linked groups
		 * share one page and number their slices on across it; default = `sliceIndex`
		 */
		pageIndex?: number;
		allSlices?: SliceModel[];
	} = $props();

	const toonId = $derived(page.params.toonId);
	// a sensitive slice while the reader hides sensitive content (members: their account setting, on by default; guests and
	// a session still loading: always): blurred, its sounds silent, a small note on it (agents/ITEMS.md "Sensitive slices").
	// Reactive: switching the setting off on the account page shows the slice.
	const session = getSessionContext();
	const member = $derived(!!session?.isLoggedIn);
	const blurred = $derived(isSensitive(slice) && hidesSensitive(member ? session.me?.user : null));
	/** the slice's audio events without its own sounds while blurred (the chapter's music levels still follow the slices) */
	const audioEvents = $derived(blurred ? (slice.audio?.music ? { music: slice.audio.music } : undefined) : slice.audio);
	// the image for the reader's media quality (1x / 2x / 3x = low / medium / high); 1x until the level is known
	const imageUrl = $derived.by(() => {
		const hasImageVariants = Object.values(slice.imageVariants ?? {}).some((file) => !!file);
		const level = hasImageVariants ? mediaQuality.level : null;
		return asset(`${toonId}/${level ? pickQualityVariant({ src: slice.image, variants: slice.imageVariants }, level) : slice.image}`);
	});
	// the file for the reader's media quality (lib/media-quality.svelte.ts); a video without variants is just its `src`
	const hasVariants = $derived(Object.values(slice.video?.variants ?? {}).some((file) => !!file));
	const videoUrl = $derived.by(() => {
		const video = slice.video;
		if (!video?.src || blurred) return undefined;
		if (!hasVariants) return asset(`${toonId}/${video.src}`);
		const level = mediaQuality.level;
		return level ? asset(`${toonId}/${pickQualityVariant(video, level)}`) : undefined;
	});
	// "Automatic" without the Network Information API: measure with the first media (its low variant)
	$effect(() => {
		const video = slice.video;
		if (video?.src && !blurred && hasVariants && mediaQuality.needsMeasure) void mediaQuality.measure(asset(`${toonId}/${pickQualityVariant(video, 'low')}`));
	});
	// the same for an image with 2x / 3x: measured with its 1x file
	$effect(() => {
		if (slice.image && Object.values(slice.imageVariants ?? {}).some((file) => !!file) && mediaQuality.needsMeasure) void mediaQuality.measure(asset(`${toonId}/${slice.image}`));
	});
	// the player video's controls (@scutoons/reader PlayerOverlay) in the reader's language
	const playerLabels: Partial<PlayerLabels> = {
		play: m['player.play'](),
		pause: m['player.pause'](),
		replay: m['player.replay'](),
		mute: m['player.mute'](),
		unmute: m['player.unmute'](),
		soundOn: m['player.sound_on'](),
		fullscreen: m['player.fullscreen'](),
		exitFullscreen: m['player.exit_fullscreen'](),
		seek: m['player.seek'](),
		skip: m['player.skip']()
	};
	/** the first slices load right away, the rest lazily (agents/PERFORMANCE.md P2) */
	const EAGER_SLICES = 2;
	// chapter files are loaded per locale, so the UI locale is the language of the text
	const lang = getLocale();

	// placed items (Slice.items): icons from the item catalog; a tap does what the creator chose (lib/visual-novel/slice-items.ts)
	const game = getGameContext();
	const vn = getVisualNovelContext();
	const itemInfo: ResolveSliceItem = (item) => {
		const inventory = getCurrentInventory();
		const def = inventory?.item(item.item);
		return def?.icon ? { src: asset(def.icon), label: inventory!.nameOf({ item_id: item.item, variant: null }) } : null;
	};
	/** bumped after a pickup: a respawning item's "taken on this visit" isn't reactive state */
	let taken = $state(0);
	const itemHidden = (item: SliceItem) => {
		void taken;
		return item.action === 'pickup' ? isTaken(slice, item, game) : false;
	};
	/** placements whose events already ran on this page view (`once`, default true) */
	const ran = new Set<string>();
	async function onItemTap(item: SliceItem) {
		if (!game) return;
		if (item.action === 'pickup') {
			const result = await pickUpSliceItem(slice, item, game);
			taken++;
			// tell the reader why it stayed (or that it was theirs already); a tap while one is pending says nothing
			if (!result.ok && result.error) getCurrentInventory()?.refused(item.item, result.error);
		} else if (item.action === 'events' && vn) {
			const once = item.once ?? true;
			if (once && ran.has(item.id)) return;
			if (once) ran.add(item.id);
			const destination = await runSliceItemEvents(item, game, vn);
			if (destination) goto(destination);
		}
	}

	// text boxes revealed by onomatopoeia events (params.dialog): hidden until then, then they play the event's effect
	// the slice's events and its placed items' tap events
	const sliceRefs = $derived(
		new Set(
			[...(slice.events ?? []), ...(slice.items ?? []).flatMap((it) => it.events ?? [])].flatMap((e) =>
				e.type === 'showOnomatopeia' && typeof e.params?.dialog === 'string' && e.params.dialog ? [e.params.dialog] : []
			)
		)
	);
	const dialogHidden = (dialog: Dialog) => !!vn?.dialogHidden(dialog.id, sliceRefs);
	const dialogPlay = (dialog: Dialog): DialogPlay | null => {
		const active = vn?.dialogPlaying(dialog.id);
		if (!active) return null;
		return {
			key: active.id,
			effect: active.effect,
			// the same limits as the event's own onomatopoeia (@scutoons/reader Onomatopoeia)
			duration: sanitizeNumber(active.duration ?? 2000, 2000, 0, 5000),
			delay: sanitizeNumber(active.delay ?? 0, 0, 0, 5000),
			onComplete: active.onComplete
		};
	};
</script>

<Slice
	{slice}
	data-slice-index={pageIndex ?? sliceIndex}
	src={imageUrl}
	videoSrc={videoUrl}
	eager={sliceIndex < EAGER_SLICES}
	autoplayPlayer
	{playerLabels}
	skippable
	{lang}
	{itemInfo}
	{itemHidden}
	onitemtap={onItemTap}
	{dialogHidden}
	{dialogPlay}
	{blurred}
	{@attach fromAction(sliceAudio, () => ({
		events: audioEvents,
		index: sliceIndex,
		slices: allSlices,
		player: audioPlayer()
	}))}
>
	{#snippet before()}
		{#if !blurred}
			<Samples samples={slice.audio?.audio?.samples} />
		{/if}
		<Triggers events={slice.events} />
	{/snippet}
	{#snippet children()}
		{#if blurred}
			<!-- the note: members can switch the setting off in their account, guests always hide (no link, less UI) -->
			{#if member}
				<a class="sensitive-note" href={localizeHref('/account')} title={m['chapter.sensitive_hidden_hint']()} data-testid="sensitive-note">{m['chapter.sensitive_hidden']()}</a>
			{:else}
				<span class="sensitive-note" data-testid="sensitive-note">{m['chapter.sensitive_hidden']()}</span>
			{/if}
		{/if}
	{/snippet}
</Slice>

<style>
	/* a caption-size pill in the middle of the blurred slice */
	.sensitive-note {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		z-index: 2;
		padding: 0.25rem 0.75rem;
		border-radius: 999rem;
		background: var(--ink);
		color: var(--paper);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		white-space: nowrap;
		text-decoration: none;
	}
	a.sensitive-note:hover {
		text-decoration: underline;
	}
</style>
