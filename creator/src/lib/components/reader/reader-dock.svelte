<script lang="ts" module>
	let mounts = 0;
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import { CloseButton, DockFold, Segmented } from '@scutoons/ui';
	import { isReaderReady, readerShowMessage, type ReaderShow } from '@scutoons/content';
	import { VIEWER_URL } from '$lib/config.ts';
	import { draftSaves } from '$lib/editor/draft-saves.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Reader in the shell's right dock (BACKLOG 10.49, UI-CONCEPTS E.6, agents/DOCKING.md "The Reader panel"): the
	 * viewer's draft preview (`?draft=1`) in a frame, a phone (390 × 844) or a desktop window (1440 × 810) scaled to the
	 * panel. Docked with `kind: 'reader'`, so group → group and chapter → chapter keep this instance (and the frame) – the
	 * place moves by message, the frame only reloads after a stored draft of this toon (any page, Ask too: draft-saves) or
	 * a change of the language / the screen. Messages (`@scutoons/content` reader-link.ts): the frame says "ready" →
	 * this sends "show <group / slice>" – then on every new place. Both checked: only "ready" from the frame's own window
	 * and the viewer's origin; "show" only to the viewer's origin. The frame signs in with the shared session cookie (the
	 * creator and the viewer are one site: scutoons.com, locally localhost) – no token in the address or in a message.
	 */
	let { place, locale, onclose }: { place: ReaderShow; locale: string; onclose: () => void } = $props();

	const VIEWER_ORIGIN = new URL(VIEWER_URL).origin;

	type Screen = 'phone' | 'desktop';
	/**
	 * the emulated screens in CSS px: the frame's viewport must be exactly the device's, whatever the creator's own root
	 * size (the reader lays out by its px breakpoints) – then scaled to the panel (a device size, not a UI size)
	 */
	const SCREENS: Record<Screen, { width: number; height: number }> = { phone: { width: 390, height: 844 }, desktop: { width: 1440, height: 810 } };
	const SCREEN_KEY = 'sc-reader-screen';
	const stored = (): Screen => {
		try {
			return localStorage.getItem(SCREEN_KEY) === 'desktop' ? 'desktop' : 'phone';
		} catch {
			return 'phone';
		}
	};
	let screen = $state<Screen>(stored());
	function pick(next: Screen) {
		screen = next;
		try {
			localStorage.setItem(SCREEN_KEY, next);
		} catch {
			// storage blocked: the pick lasts for this page view
		}
	}

	let frame = $state<HTMLIFrameElement>();
	let boxWidth = $state(0);
	let boxHeight = $state(0);
	const device = $derived(SCREENS[screen]);
	const scale = $derived(boxWidth && boxHeight ? Math.min(boxWidth / device.width, boxHeight / device.height) : 0);

	/** the frame's address: the chapter in the reader's draft preview (the place itself comes by message) */
	const address = () => `${VIEWER_URL}/${locale}/${place.toon}/chapter/${place.chapter}?draft=1`;
	/** the frame said "ready" since its last (re)load */
	let ready = $state(false);
	/** counts the frame's loads (the headless check: a save reloads it) */
	let loads = $state(0);
	function reload() {
		if (!frame) return;
		ready = false;
		frame.src = address();
	}
	// (re)load on mount, a new language or screen – keyed by value: the dock hands new props on every change of the
	// page's place, the frame must not reload for those
	const frameKey = $derived(`${locale}|${screen}`);
	$effect(() => {
		void frameKey;
		if (frame) untrack(reload);
	});
	// a stored draft of this toon: reload (a save of several files at once: one reload)
	let seenSaves = draftSaves.seq;
	$effect(() => {
		const seq = draftSaves.seq;
		if (seq === seenSaves) return;
		seenSaves = seq;
		if (draftSaves.toon && draftSaves.toon !== untrack(() => place.toon)) return;
		const timer = setTimeout(reload, 300);
		return () => clearTimeout(timer);
	});

	/** "show" to the frame – only to the viewer's origin (a frame navigated elsewhere never gets it) */
	function show() {
		frame?.contentWindow?.postMessage(readerShowMessage($state.snapshot(place)), VIEWER_ORIGIN);
	}
	// a new place: "show" (by value – the same place handed again is no new place)
	const placeKey = $derived(`${place.toon}|${place.chapter}|${place.group}|${place.slice}`);
	$effect(() => {
		void placeKey;
		if (ready) untrack(show);
	});
	$effect(() => {
		const onmessage = (event: MessageEvent) => {
			if (event.origin !== VIEWER_ORIGIN || !frame || event.source !== frame.contentWindow || !isReaderReady(event.data)) return;
			ready = true;
			loads++;
			show();
		};
		window.addEventListener('message', onmessage);
		return () => window.removeEventListener('message', onmessage);
	});

	const mount = ++mounts;
</script>

<section class="reader-dock" data-testid="reader-dock" data-mount={mount} data-loads={loads} data-ready={ready}>
	<header>
		<h2>{m['creator.reader']()}</h2>
		<Segmented
			label={m['creator.strip_preview_screen']()}
			value={screen}
			onchange={pick}
			options={[
				{ value: 'phone', icon: 'smartphone', title: m['creator.strip_preview_phone'](), testid: 'reader-phone' },
				{ value: 'desktop', icon: 'monitor', title: m['creator.strip_preview_desktop'](), testid: 'reader-desktop' }
			]}
		/>
		<DockFold label={m['creator.fold_panel']()} maximizeLabel={m['creator.maximize_panel']()} restoreLabel={m['creator.restore_panel']()} />
		<CloseButton label={m['creator.close']()} onclick={onclose} />
	</header>
	<div class="box">
		<div class="fit" bind:clientWidth={boxWidth} bind:clientHeight={boxHeight}>
			<div class="device" style:width="{device.width * scale}px" style:height="{device.height * scale}px">
				<iframe
					bind:this={frame}
					title={m['creator.reader_frame']()}
					style:width="{device.width}px"
					style:height="{device.height}px"
					style:transform="scale({scale})"
					referrerpolicy="no-referrer"
					allow="autoplay; fullscreen"
					data-testid="reader-frame"
				></iframe>
			</div>
		</div>
	</div>
</section>

<style>
	/* fills the dock edge to edge: the header, then the screen centred in the rest */
	.reader-dock {
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: 0;
		flex: 1;
	}
	.box {
		flex: 1;
		min-height: 0;
		padding: var(--space-05);
		overflow: hidden;
	}
	/* the room the screen is scaled into (the box minus its padding) */
	.fit {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: flex-start;
		justify-content: center;
	}
	/* the screen's edge drawn outside it (an outline takes no room from the scaled frame) */
	.device {
		flex: none;
		overflow: hidden;
		outline: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	iframe {
		display: block;
		border: 0;
		transform-origin: 0 0;
	}
</style>
