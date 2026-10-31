<script lang="ts">
	// /live/deck (#198, owner 2026-10-31: "On the S22 I want the same visual as the Rust stage, just in wasm … the
	// character switcher could be superimposed … with regular Svelte or HTML UI"; "I'd rather ship it in the reader … in
	// the PWA"): a phone joins the Cathedral of decks as a being. The being and the cathedral fill the screen – deck-space,
	// the native stage's own code as wasm (@scutoons/live-deck's SpaceStage); over it the character chip and the
	// controls. Sound: live-core. The relay stream: as the lab's deck (deck.graph / deck.values). Unlisted (no link in
	// the navigation); everyone may open it (lib/live-deck.ts `mayJoin` – the one place a gate goes later). A member
	// joins under their name with their avatar's character; a guest under the guest name. The relay (#199): the hosted
	// one, wss://live.scutoons.com/live/<code> (VITE_LIVE_URL), the code from the path `/live/deck/<code>` or `?code=`
	// (default CATH); `?relay=ws://…/live/CODE` joins a local relay instead. The conductor's scene sync (#195): the deck
	// fetches what it names from the CDN and answers; the status line shows it. The controls (#209): the knobs (the
	// panel's dials, the being's – they face the camera), two corner sticks over the 3D, MIDI learn; what the sticks
	// drive is the selection's (the last tapped knob / the being / nothing = the master section) – the logic is
	// deck-space's (Hands → DeckControls, wasm), this page only bridges the DOM, Web MIDI and the pointer to it.
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { Deck, FrameStats, Hands, MELODY, PARAMS, SpaceStage, charactersOf, loadControls, pushLine, type Hand } from '@scutoons/live-deck';
	import LiveWorker from '@scutoons/live-core/worker?worker';
	import workletUrl from '@scutoons/live-core/worklet?worker&url';
	import { API_URL, CDN_SERVER, LIVE_URL } from '$lib/config.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { CHARACTER_KEY, WASM_BASE, characterOf, mayJoin, nickOf, preferOf, relayOf } from '$lib/live-deck.ts';
	import CharacterChip from '$lib/components/live-deck/character-chip.svelte';
	import DeckPanel from '$lib/components/live-deck/deck-panel.svelte';
	import DeckSticks from '$lib/components/live-deck/deck-sticks.svelte';
	import type { Stats } from '$lib/components/live-deck/deck-stats.svelte';
	import { DeckControlsState } from '$lib/live-deck-controls.svelte.ts';

	const session = getSessionContext();
	const cdn = `${CDN_SERVER}/`;
	let host: HTMLDivElement | undefined = $state();
	// the controls show the scene's values until the deck is there
	let values = $state<Record<string, number>>(Object.fromEntries(Object.entries({ ...PARAMS, ...MELODY }).map(([k, s]) => [k, s.value])));
	let character = $state('');
	let characters = $state<{ id: string; name?: Record<string, string> }[]>([]);
	let mood = $state('neutral');
	let face = $state('');
	let started = $state(false);
	let starting = $state(false);
	let relayStatus = $state('');
	let syncStatus = $state('');
	let spaceStatus = $state('');
	let hud = $state('');
	// #208: the warning a few seconds after a dropout; #209: the collapsed bar's statistics (#188: the dropouts over the
	// last minute, the worker's peak render load)
	let audioAlert = $state('');
	let stats = $state<Stats>({ fps: 0, cpu: 0, peak: 0, dropouts: null, load: 0, relay: '', backend: '' });
	const frames = new FrameStats();
	// #209: the controls – the selection the sticks follow, MIDI learn, the mappings per deck
	const specs = { ...PARAMS, ...MELODY };
	const ctl = new DeckControlsState(specs);
	ctl.onSelect = (name) => space?.view.select(name);
	let threeD = $state(true);
	let deck: Deck | null = null;
	let space: SpaceStage | null = null;

	const store = {
		get: () => {
			try {
				return localStorage.getItem(CHARACTER_KEY);
			} catch {
				return null;
			}
		},
		set: (v: string) => {
			try {
				localStorage.setItem(CHARACTER_KEY, v);
			} catch {
				/* private mode */
			}
		}
	};

	const status = $derived([syncStatus, spaceStatus, hud].filter(Boolean).join(' · '));

	function showFace() {
		const p = deck?.portraits[mood] ?? deck?.portraits.neutral ?? Object.values(deck?.portraits ?? {})[0];
		face = p ? cdn + p.path : '';
	}

	/** the session's first answer (a member's name and avatar), at most a moment */
	async function settled() {
		for (let i = 0; i < 40 && !session.ready; i++) await new Promise((r) => setTimeout(r, 50));
	}

	onMount(() => {
		let alive = true;
		let raf = 0;
		const audio = setInterval(() => {
			const core = deck?.core;
			if (!core) return;
			const s = core.stats();
			stats.dropouts = s.underrunsPerMinute;
			stats.load = Math.round((100 * s.renderPeakUs) / Math.max(1, s.budgetUs));
			audioAlert = core.glitching() ? m['live_deck.dropout_alert']() : '';
		}, 250);
		const bar = setInterval(() => {
			stats.fps = frames.fps;
			stats.cpu = frames.cpu;
			stats.peak = frames.peak;
			stats.relay = relayStatus;
			stats.backend = space ? space.backend : spaceStatus;
		}, 500);
		void (async () => {
			await settled();
			if (!alive || !mayJoin(session.me)) return;
			const q = page.url.searchParams;
			const relay = relayOf(q, LIVE_URL, page.params.code);
			character = characterOf(q, session.me, store.get());
			deck = new Deck({ relay, character, nick: nickOf(session.me, m['live_deck.guest']()), api: API_URL, cdn, onStatus: (s) => (relayStatus = s), onSynced: (r) => (syncStatus = pushLine(r)) });
			values = { ...deck.value };
			deck.onValue = (p: string, v: number, hand: Hand) => {
				if (hand === 'being' || values[p] !== v) values[p] = v;
			};
			deck.onMood = (md) => {
				mood = md;
				showFace();
			};
			try {
				ctl.hands = new Hands(await loadControls(WASM_BASE, Object.keys(specs)), deck, specs);
				ctl.load(character);
				ctl.choose('', false);
			} catch (e) {
				console.warn('live deck: no controls (the wasm is missing)', e);
			}
			await deck.pickCharacter(character);
			showFace();
			charactersOf(API_URL, character).then((list) => (characters = list));
			if (!alive || !host) return;
			try {
				space = await SpaceStage.start({ host, base: WASM_BASE, deck, api: API_URL, cdn, prefer: preferOf(q), onStatus: (s) => (spaceStatus = s) });
			} catch (e) {
				threeD = false;
				spaceStatus = m['live_deck.no_3d']({ reason: (e as Error).message ?? String(e) });
			}
			const frame = () => {
				if (!alive || !deck) return;
				const t0 = performance.now();
				const now = t0 / 1000;
				deck.tick(now);
				try {
					space?.frame(now);
					hud = space?.view.hud_line() ?? '';
					// #209: a tap on the being selects (a knob, the being, empty space = the master section)
					const tap = space?.view.take_tap();
					if (tap !== undefined) ctl.choose(tap, false);
				} catch (e) {
					spaceStatus = m['live_deck.no_3d']({ reason: (e as Error).message });
					space?.stop();
					space = null;
					threeD = false;
				}
				frames.push(t0, performance.now() - t0);
				raf = requestAnimationFrame(frame);
			};
			raf = requestAnimationFrame(frame);
		})();
		return () => {
			alive = false;
			clearInterval(audio);
			clearInterval(bar);
			cancelAnimationFrame(raf);
			ctl.close();
			space?.stop();
			deck?.close();
			void deck?.core?.stop?.();
		};
	});

	async function pick(id: string) {
		if (!deck) return;
		character = id;
		store.set(id);
		await deck.pickCharacter(id);
		space?.setBeing();
		ctl.load(id);
		ctl.choose('', true);
		showFace();
	}

	async function start() {
		if (!deck || starting) return;
		starting = true;
		try {
			await deck.start({ worker: new LiveWorker(), wasm: new URL(`${WASM_BASE}live_core.wasm`, location.href), worklet: workletUrl });
			started = true;
		} catch (e) {
			relayStatus = (e as Error).message;
		} finally {
			starting = false;
		}
	}
</script>

<svelte:head>
	<title>{m['live_deck.title']()} · SCUTOONS</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<div class="deck" data-testid="live-deck">
	<div class="stage" bind:this={host}>
		{#if !threeD && face}<img class="face" src={face} alt="" crossorigin="anonymous" />{/if}
	</div>
	<div class="top">
		{#if character}
			<CharacterChip current={character} {characters} {face} {mood} onpick={pick} />
		{/if}
	</div>
	<DeckSticks labels={ctl.labels} ondown={(s) => ctl.hands?.stickDown(s)} onmove={(s, x, y) => ctl.hands?.stickMove(s, x, y)} onup={(s) => ctl.hands?.stickUp(s)} />
	<div class="bottom">
		<DeckPanel
			{values}
			{started}
			{starting}
			{stats}
			{status}
			alert={audioAlert}
			sticks={ctl.line}
			selected={ctl.knob}
			learn={ctl.learn}
			learning={ctl.learning}
			midi={ctl.midiStatus}
			mapped={ctl.mapped}
			onstart={start}
			onset={(p, v) => deck?.setParam(p, v)}
			onpick={ctl.pick}
			onlearn={ctl.toggleLearn}
			ontouch={(on) => deck && (deck.touch = on)}
		/>
	</div>
</div>

<style>
	.deck {
		position: fixed;
		inset: 0;
		background: var(--ink);
		color: var(--paper);
		overflow: hidden;
	}
	.stage {
		position: absolute;
		inset: 0;
	}
	.face {
		position: absolute;
		inset: 20% 15%;
		width: 70%;
		height: 60%;
		object-fit: contain;
	}
	/* the chip between the menu's round toggles (top left / right) */
	.top {
		position: absolute;
		top: calc(0.5rem + env(safe-area-inset-top));
		left: 50%;
		transform: translateX(-50%);
	}
	.bottom {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		pointer-events: none;
	}
</style>
