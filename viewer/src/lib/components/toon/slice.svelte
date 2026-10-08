<script lang="ts">
	import { fromAction } from 'svelte/attachments';
	import { hasSliceLayers, hidesSensitive, isAreaTrigger, isSensitive, pickQualityVariant, sanitizeNumber, Trigger, type Dialog, type Slice as SliceModel, type SliceItem, type SliceLayerRef } from '@scutoons/content';
	import { Slice, liftCue, nudgeCue, sliceAudio, type DialogPlay, type PlayerLabels, type ResolveSliceItem, type Shape } from '@scutoons/reader';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { getCurrentInventory } from '$lib/contexts/inventory.svelte.ts';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { isTaken, markNudged, pickUpSliceItem, runSliceItemEvents } from '$lib/visual-novel/slice-items.ts';
	import { idsOf, isRunDue, recordRun, type RunMemory } from '$lib/visual-novel/repeat.ts';
	import { getGroupScope } from '$lib/visual-novel/group-scope.ts';
	import { asset, toonUrl } from '$lib/utils/cdn.ts';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { audioPlayer } from '$lib/actions/audio/index.ts';
	import { mediaQuality } from '$lib/media-quality.svelte.ts';
	import { getReaderExpressions } from '$lib/expressions/resolver.svelte.ts';
	import Samples from './audio/samples.svelte';
	import SampleTrigger from './audio/sample-trigger.svelte';
	import Triggers from './vn/triggers.svelte';
	import ClickTrigger from './vn/triggers/click.svelte';
	import HoverTrigger from './vn/triggers/hover.svelte';

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
	/**
	 * the author ordered the slice's layer stack (BACKLOG 10.54, @scutoons/content layers.ts): its click / hover areas
	 * (events with a rect, samples with an area) are drawn in the stack and take the pointer there – the front-most under
	 * the pointer reacts, not the ones it covers. None ordered: the old way (the areas under the stack, the slice listens:
	 * every area under the pointer reacts).
	 */
	const layered = $derived(hasSliceLayers(slice));
	/**
	 * the slice's audio events without its own sounds while blurred (the chapter's music levels still follow the slices);
	 * the slice plays its first sample (`sliceAudio`) – unless it is an area of the ordered stack (it plays itself)
	 */
	const audioEvents = $derived.by(() => {
		if (blurred) return slice.audio?.music ? { music: slice.audio.music } : undefined;
		if (layered && isAreaTrigger(slice.audio?.audio?.samples?.[0]?.trigger)) return { ...slice.audio, audio: undefined };
		return slice.audio;
	});
	/** an area's sample (the ordered stack): played by the area itself */
	const playSample = (index: number) => {
		const sample = slice.audio?.audio?.samples?.[index];
		if (sample) audioPlayer()?.triggerSample(sample);
	};
	// the image for the reader's media quality (1x / 2x / 3x = low / medium / high); 1x until the level is known
	const imageUrl = $derived.by(() => {
		const hasImageVariants = Object.values(slice.imageVariants ?? {}).some((file) => !!file);
		const level = hasImageVariants ? mediaQuality.level : null;
		return toonUrl(toonId, level ? pickQualityVariant({ src: slice.image, variants: slice.imageVariants }, level) : slice.image);
	});
	// the file for the reader's media quality (lib/media-quality.svelte.ts); a video without variants is just its `src`
	const hasVariants = $derived(Object.values(slice.video?.variants ?? {}).some((file) => !!file));
	const videoUrl = $derived.by(() => {
		const video = slice.video;
		if (!video?.src || blurred) return undefined;
		if (!hasVariants) return toonUrl(toonId, video.src);
		const level = mediaQuality.level;
		return level ? toonUrl(toonId, pickQualityVariant(video, level)) : undefined;
	});
	// "Automatic" without the Network Information API: measure with the first media (its low variant)
	$effect(() => {
		const video = slice.video;
		if (video?.src && !blurred && hasVariants && mediaQuality.needsMeasure) void mediaQuality.measure(toonUrl(toonId, pickQualityVariant(video, 'low')));
	});
	// the same for an image with 2x / 3x: measured with its 1x file
	$effect(() => {
		if (slice.image && Object.values(slice.imageVariants ?? {}).some((file) => !!file) && mediaQuality.needsMeasure) void mediaQuality.measure(toonUrl(toonId, slice.image));
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
	// `{…}` in the text boxes, answered for this reader (agents/EXPRESSIONS.md); visits counted once per page view
	const expressions = getReaderExpressions()?.textBoxes;

	// placed items (Slice.items): icons from the item catalog; a tap does what the creator chose (lib/visual-novel/slice-items.ts)
	const game = getGameContext();
	const vn = getVisualNovelContext();
	// the slice's group (linked groups share a page): its script for diverts, its text boxes; `done` = above the `?label=`
	// target of a chain – read already: its boxes show as they are (owner 2026-10-06)
	const group = getGroupScope();
	const scope = group?.scope ?? '';
	const itemInfo: ResolveSliceItem = (item) => {
		const inventory = getCurrentInventory();
		const def = inventory?.item(item.item);
		// a placed variant shows its own picture and name (a colour of the sunglasses)
		const variant = item.variant ? def?.variants?.find((v) => v.id === item.variant) : undefined;
		const icon = variant?.icon || def?.icon;
		return icon ? { src: asset(icon), label: inventory!.nameOf({ item_id: item.item, variant: variant ? item.variant! : null }) } : null;
	};
	/** bumped after a pickup: a respawning item's "taken on this visit" isn't reactive state */
	let taken = $state(0);
	const itemHidden = (item: SliceItem) => {
		void taken;
		return item.action === 'pickup' ? isTaken(slice, item, game) : false;
	};
	/** how often a placement's tap events run: its `repeat` over the reader's history (visual-novel/repeat.ts) */
	const tapMemory = new Map<string, RunMemory>();
	async function onItemTap(item: SliceItem, _index: number, el: HTMLElement) {
		if (!game) return;
		if (item.action === 'pickup') {
			// agents/FX.md: it jumps up at once, then flies into the bag (the inventory button) or shakes and stays
			const lift = liftCue(el, item.rotation ?? 0);
			const result = await pickUpSliceItem(slice, item, game);
			// the sound in the bag: this placement's, else the item's (catalog), else pling; `none` = silent
			const sound = item.sound ?? getCurrentInventory()?.item(item.item)?.sound;
			if (result.ok) await lift.fly(document.querySelector('.inventory-toggle'), sound === 'none' ? null : (sound ?? 'pling'));
			else if (result.error) lift.refuse();
			else lift.drop(); // a second tap while the first is still asking: nothing
			taken++;
			// tell the reader why it stayed (or that it was theirs already); a tap while one is pending says nothing
			if (!result.ok && result.error) getCurrentInventory()?.refused(item.item, result.error);
		} else if (item.action === 'events' && vn) {
			const ids = idsOf(item.events);
			const memory = tapMemory.get(item.id) ?? { ran: false };
			tapMemory.set(item.id, memory);
			if (!isRunDue(item.repeat, ids, game, memory)) return;
			recordRun(ids, game, memory);
			const destination = await runSliceItemEvents(item, game, vn, group);
			if (destination) goto(destination);
		}
	}

	/** "you can tap me" (agents/FX.md): an untouched tappable item wiggles – one per slice and page view */
	function onItemIdle(item: SliceItem, _index: number, el: HTMLElement, shape: Shape | null) {
		if (blurred || !markNudged(slice.image)) return;
		nudgeCue(el, item.rotation ?? 0, shape);
	}

	// text boxes revealed by showTextBox events (params.dialog): hidden until then, then they play the event's effect
	// the slice's events and its placed items' tap events
	const sliceRefs = $derived(
		new Set(
			[...(slice.events ?? []), ...(slice.items ?? []).flatMap((it) => it.events ?? [])].flatMap((e) =>
				e.type === 'showTextBox' && typeof e.params?.dialog === 'string' && e.params.dialog ? [e.params.dialog] : []
			)
		)
	);
	const dialogHidden = (dialog: Dialog) => !group?.done && !!vn?.dialogHidden(dialog.id, sliceRefs, scope);
	const dialogPlay = (dialog: Dialog): DialogPlay | null => {
		const active = vn?.dialogPlaying(dialog.id, scope);
		if (!active) return null;
		return {
			key: active.id,
			effect: active.effect,
			sound: active.sound,
			shake: active.shake,
			// the limits of a showTextBox event's timing (ShowTextBoxParams)
			duration: sanitizeNumber(active.duration ?? 2000, 2000, 0, 5000),
			delay: sanitizeNumber(active.delay ?? 0, 0, 0, 5000),
			onComplete: active.onComplete
		};
	};
</script>

<!-- one click / hover area of the ordered stack (BACKLOG 10.54): an event area (its events run as one) or a sample's -->
{#snippet stackArea(ref: SliceLayerRef)}
	{#if ref.kind === 'event' && !group?.done}
		{@const events = (ref.events ?? [ref.index]).map((i) => slice.events![i])}
		{#if events[0]?.trigger?.type === Trigger.CLICK}
			<ClickTrigger {events} own />
		{:else if events[0]?.trigger?.type === Trigger.HOVER}
			<HoverTrigger {events} own />
		{/if}
	{:else if ref.kind === 'sample' && !blurred}
		<SampleTrigger trigger={slice.audio?.audio?.samples?.[ref.index]?.trigger} onplay={() => playSample(ref.index)} />
	{/if}
{/snippet}

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
	onitemidle={onItemIdle}
	{dialogHidden}
	{dialogPlay}
	{expressions}
	{blurred}
	area={layered ? stackArea : undefined}
	{@attach fromAction(sliceAudio, () => ({
		events: audioEvents,
		index: sliceIndex,
		slices: allSlices,
		player: audioPlayer()
	}))}
>
	{#snippet before()}
		{#if !blurred}
			<Samples samples={slice.audio?.audio?.samples} areas={!layered} />
		{/if}
		<Triggers events={slice.events} areas={!layered} />
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
