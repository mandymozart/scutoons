<script lang="ts">
	// #463: with nothing in the address the deck idles – the core's idle scene and nothing else.
	// /live/deck (#198, owner 2026-10-31: "On the S22 I want the same visual as the Rust stage, just in wasm … the
	// character switcher could be superimposed … with regular Svelte or HTML UI"; "I'd rather ship it in the reader … in
	// the PWA"): a phone joins the Cathedral of decks as a being. The being and the cathedral fill the screen – deck-space,
	// the native stage's own code as wasm (@scutoons/live-deck's SpaceStage); over it the round Play and the
	// controls (#460: what a member may choose comes later, from the scene's rules). Sound: live-core. The relay stream: as the lab's deck (deck.graph / deck.values). Unlisted (no link in
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
	import { Hands, SceneStage, loadControls, loadDeckState, type QualityLevel, type QualityOptions, type QualityPresets, type SpaceQuality, type Spec } from '@scutoons/live-deck';
	// #298: the engine's bring-up (the deck, the 3D, the sound, the frame loop, the rig for a mood, the statistics) is
	// @scutoons/engine-host's; this page keeps the relay, the controls and its UI
	import { createEngineHost, type EngineHost } from '@scutoons/engine-host';
	import { API_URL, CDN_SERVER, LIVE_QUIC, LIVE_URL } from '$lib/config.ts';
	import { getSessionToken } from '@scutoons/api';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { WASM_BASE, characterOf, mayJoin, nickOf, preferOf, relayOf, rigOf } from '$lib/live-deck.ts';
	import DeckPanel from '$lib/components/live-deck/deck-panel.svelte';
	import DeckQualitySheet from '$lib/components/live-deck/deck-quality-sheet.svelte';
	import DeckSticks from '$lib/components/live-deck/deck-sticks.svelte';
	import DeckStageOnly from '$lib/components/live-deck/deck-stage-only.svelte';
	import type { Stats } from '$lib/components/live-deck/deck-stats.svelte';
	import { DeckControlsState } from '$lib/live-deck-controls.svelte.ts';
	import { nowMs, type ClockState } from '@scutoons/live-deck';

	const session = getSessionContext();
	// #463, owner 2026-11-10: "the fireflies scene should play as default when nothing is induced … other need external
	// triggers. for testing we can still set those parameters with url params for now"
	const idle = !page.params.code && !['character', 'rig', 'code', 'relay', 'quic'].some((k) => page.url.searchParams.has(k));
	const cdn = `${CDN_SERVER}/`;
	let stage: HTMLDivElement | undefined = $state();
	// the controls show the scene's values once the core gave the scene (#360)
	let values = $state<Record<string, number>>({});
	// the sliders: the silent scene's look knobs (#425), or the character's rig's (#191, `?rig=1`)
	let params = $state<Record<string, Spec>>({});
	let character = $state('');
	let mood = $state('');
	let face = $state('');
	let started = $state(false);
	let starting = $state(false);
	let relayStatus = $state('');
	let syncStatus = $state('');
	let spaceStatus = $state('');
	let hud = $state('');
	// #208: the warning a few seconds after a dropout; #209: the collapsed bar's statistics (#364: deck-core's monitor –
	// its items, words and judgement)
	let audioAlert = $state('');
	let stats = $state<Stats>({ items: [], relay: '', backend: '', quality: '', down: false });
	// #298: the engine host – null until the session answered (the deck is made then)
	let engine: EngineHost | null = null;
	// #209: the controls – the selection the sticks follow, MIDI learn, the mappings per deck
	const ctl = new DeckControlsState();
	ctl.onSelect = (name) => engine?.space?.view.select(name);
	// #225: the corner sticks can be switched off in the panel (they free the knobs under them), remembered per device
	const STICKS_KEY = 'scutoons.deck.sticks';
	let sticksOn = $state(true);
	function toggleSticks() {
		sticksOn = !sticksOn;
		try {
			localStorage.setItem(STICKS_KEY, sticksOn ? '1' : '0');
		} catch {
			/* private mode */
		}
	}
	// #220: the 3D's render quality – the governor's (deck-space); the presets (the settings sheet, the same text as the
	// X1's quality file) remembered per device. The night's first build kept only { level, auto } under QUALITY_KEY: read
	// once into the presets, then gone.
	const PRESETS_KEY = 'scutoons.deck.presets';
	const QUALITY_KEY = 'scutoons.deck.quality';
	let quality = $state<SpaceQuality | null>(null);
	let presets = $state<QualityPresets | null>(null);
	let presetsText = $state('');
	/** #375: the option lists, the core's */
	let qualityOptions = $state<QualityOptions | null>(null);
	let sheetOpen = $state(false);
	let screenInfo = $state<{ max: [number, number]; display: [number, number] }>({ max: [0, 0], display: [0, 0] });
	function stored(key: string): string | null {
		try {
			return localStorage.getItem(key);
		} catch {
			return null;
		}
	}
	/** after a change: the sheet's view, the bar's, and the text kept for this device */
	function keepPresets() {
		const space = engine?.space;
		if (!space) return;
		quality = space.quality;
		presets = space.presets;
		presetsText = space.presetsText;
		qualityOptions = space.qualityOptions;
		screenInfo = space.screen;
		try {
			localStorage.setItem(PRESETS_KEY, presetsText);
			localStorage.removeItem(QUALITY_KEY);
		} catch {
			/* private mode */
		}
	}
	function loadPresets() {
		const space = engine?.space;
		if (!space) return;
		const text = stored(PRESETS_KEY);
		const old = stored(QUALITY_KEY);
		if (text) {
			const e = space.setPresetsText(text);
			if (e) console.warn('live deck: the stored quality presets', e);
		} else if (old) {
			try {
				const q = JSON.parse(old) as { level?: QualityLevel; auto?: boolean };
				if (q.level) space.setQuality(q.level, q.auto !== false);
			} catch {
				/* a broken entry: the defaults */
			}
		}
		keepPresets();
	}
	function setQuality(level: QualityLevel, auto: boolean) {
		engine?.space?.setQuality(level, auto);
		keepPresets();
	}
	function setPresets(p: QualityPresets): string {
		const e = engine?.space?.setPresets(p) ?? '';
		if (!e) keepPresets();
		return e;
	}
	function setPresetsText(text: string): string {
		const e = engine?.space?.setPresetsText(text) ?? '';
		if (!e) keepPresets();
		return e;
	}
	let threeD = $state(true);
	// #321: the tempo clock – the core's (live-core clock.rs): the picker's choice, the MIDI clock out and the strobe's
	// flash come in its frames (#370); the page has the buttons and draws
	let clock = $state<ClockState | null>(null);
	/** the picker's choices, the core's (asked once it runs) */
	let clockModes = $state<string[]>([]);
	let strobe = $state(false);
	let flash = $state(false);
	ctl.onClockMidi = (data, at) => engine?.deck?.midiIn(data, at);
	// Web MIDI opens once the core's clock needs it – it follows MIDI clock in, or its clock out goes to every output (as
	// the native deck opens a source's port when the core chose it, `Host::source_chosen`)
	const needsMidi = $derived(clock?.source === 'midi' || !!clock?.midiOut);
	$effect(() => {
		if (needsMidi) void ctl.startMidi();
	});
	// the checks read the clock (dev only): the beat at a page ms (`performance.timeOrigin + now()`) and the clock's state
	if (import.meta.env.DEV && typeof window !== 'undefined') (window as unknown as Record<string, unknown>).__scutoonsClock = () => ({ clock: engine?.deck?.clock });
	// the strobe: the core's flash (its beat edge and length, #370) drawn while this animation frame's page ms is in it
	$effect(() => {
		if (!strobe) return;
		let raf = 0;
		const frame = () => {
			raf = requestAnimationFrame(frame);
			const f = engine?.deck?.clock?.flash;
			const t = nowMs();
			const on = !!f && t >= f.atMs && t < f.atMs + f.ms;
			if (on !== flash) flash = on;
		};
		raf = requestAnimationFrame(frame);
		return () => {
			cancelAnimationFrame(raf);
			flash = false;
		};
	});

	const status = $derived([stats.backend, syncStatus, spaceStatus, hud].filter(Boolean).join(' · '));

	function showFace() {
		const deck = engine?.deck;
		const p = deck?.face(mood);
		face = p ? cdn + p.path : '';
	}

	/** the session's first answer (a member's name and avatar), at most a moment */
	async function settled() {
		for (let i = 0; i < 40 && !session.ready; i++) await new Promise((r) => setTimeout(r, 50));
	}

	onMount(() => {
		if (idle) {
			let alive = true;
			let scene: SceneStage | null = null;
			const q = page.url.searchParams;
			// #465: ?outlet=<id> plays the Live outlet (?live=<relay>, else the hosted show; ?locale=). #478: it joins under the
			// signed-in reader's name (the session's first answer, at most a moment) and asks its proof with the reader's
			// sign-in – the cookie, or the stored token where the browser refuses cookies
			const outlet = q.has('outlet');
			void (async () => {
				if (outlet) await settled();
				if (!alive || !stage) return;
				try {
					const s = await SceneStage.start({ host: stage, base: WASM_BASE, api: API_URL, cdn, user: () => (session.isLoggedIn ? { bearer: getSessionToken() } : null), mayReject: () => session.roles.includes('live'), scene: q.get('scene') ?? (outlet ? 'outlet' : ''), prefer: preferOf(q), outlet: outlet ? { relay: q.get('live') ?? '', name: q.get('outlet') ?? '', locale: q.get('locale') ?? 'de', reader: nickOf(session.me, '') } : undefined });
					if (alive) scene = s;
					else s.stop();
				} catch (e) {
					console.warn('live deck: no idle scene', e);
				}
			})();
			return () => {
				alive = false;
				scene?.stop();
			};
		}
		try {
			sticksOn = localStorage.getItem(STICKS_KEY) !== '0';
		} catch {
			/* private mode: on */
		}
		let alive = true;
		// #208: the warning a few seconds after a dropout; #364: the monitor's report – the host samples it, the bar draws it
		const audio = setInterval(() => {
			const s = engine?.snapshot().stats;
			if (!s) return;
			stats.items = s.bar;
			audioAlert = s.alert ? m['live_deck.dropout_alert']() : '';
		}, 250);
		const bar = setInterval(() => {
			const space = engine?.space;
			stats.relay = relayStatus;
			stats.backend = space ? space.backend : spaceStatus;
			quality = space?.quality ?? null;
			if (space && sheetOpen) screenInfo = space.screen;
			stats.quality = quality ? `${quality.level} ${quality.scale.toFixed(2)}×${quality.auto ? '' : ' FIX'}` : '';
			stats.down = !!quality && quality.level !== quality.ceiling;
		}, 500);
		// #370: the clock's frame as the core left it – its choice, the MIDI clock out, the tempo line's beat
		const clockFrames = setInterval(() => {
			clock = engine?.deck?.clock ?? null;
			if (clock && !clockModes.length) clockModes = engine?.deck?.state?.clock_modes() ?? [];
		}, 50);
		void (async () => {
			await settled();
			if (!alive || !mayJoin(session.me)) return;
			const q = page.url.searchParams;
			character = characterOf(q, session.me);
			// #191: `?rig=1` plays the character's rig (and its bundle while it has one) instead of the silent scene (#425)
			const host = createEngineHost({
				api: API_URL,
				cdn,
				wasmBase: WASM_BASE,
				character,
				nick: nickOf(session.me, m['live_deck.guest']()),
				relay: relayOf(q, LIVE_URL, page.params.code, LIVE_QUIC),
				prefer: preferOf(q),
				rigs: rigOf(q),
				onStatus: (s) => (relayStatus = s),
				onSynced: (_, line) => (syncStatus = line)
			});
			engine = host;
			host.hooks = {
				space: () => loadPresets(),
				threeD: (ok, reason) => {
					threeD = ok;
					hud = '';
					spaceStatus = m['live_deck.no_3d']({ reason });
				},
				spaceStatus: (s) => (spaceStatus = s),
				tick: () => {
					ctl.poll();
					hud = host.space ? host.hud : '';
				},
				// #209: a tap on the being selects (a knob, the being, empty space = the master section)
				tap: (name) => ctl.choose(name, false),
				rig: (_played, why) => {
					const deck = host.deck;
					if (!deck) return;
					if (why !== 'play') return;
					params = deck.params;
					values = { ...deck.value };
					// the corner sticks follow the rig's knobs (or the silent scene's again)
					ctl.useParams(deck.params, deck.sticks);
				},
				// #191: the mood's variant of the rig (its presets, its values) is already played – the sliders follow by the mood's hand
				mood: (md) => {
					mood = md;
					showFace();
				},
				value: (p, v, hand) => {
					if (hand === 'being' || values[p] !== v) values[p] = v;
				}
			};
			const deck = host.init();
			deck.onMidiOut = (bytes, at) => ctl.midiSend(bytes, at);
			values = { ...deck.value };
			try {
				// #360: the silent scene (no rig), played by the core (deck-space as wasm – no GPU needed): its knobs are the sliders
				// #367, #368: the deck's state – the silent scene, the mood ladder, the rig's lifecycle (deck-space as wasm, no GPU)
				deck.useState(await loadDeckState(WASM_BASE));
				params = deck.params;
				values = { ...deck.value };
				const specs = deck.params;
				ctl.hands = new Hands(await loadControls(WASM_BASE), deck, specs);
				ctl.useParams(specs, deck.sticks);
				ctl.load(character);
				ctl.choose('', false);
			} catch (e) {
				console.warn('live deck: no controls (the wasm is missing)', e);
			}
			await deck.pickCharacter(character);
			showFace();
			if (!alive || !stage) return;
			await host.attach(stage);
		})();
		return () => {
			alive = false;
			clearInterval(audio);
			clearInterval(bar);
			clearInterval(clockFrames);
			ctl.close();
			engine?.dispose();
			engine = null;
		};
	});

	/** #356: the sound's measurement to the clipboard (the owner pastes it into the issue), else the phone's share sheet;
	 *  the console has it too */
	async function copyStats(): Promise<boolean> {
		if (!engine) return false;
		const text = engine.report();
		console.info(text);
		try {
			await navigator.clipboard.writeText(text);
			return true;
		} catch (e) {
			console.warn('live deck: the clipboard refused the report', e);
		}
		try {
			await navigator.share({ title: 'SCUTOONS deck sound report', text });
			return true;
		} catch {
			return false;
		}
	}

	async function start() {
		if (!engine || starting) return;
		starting = true;
		started = await engine.start();
		if (!started) relayStatus = engine.snapshot().error;
		starting = false;
	}
</script>

<svelte:head>
	<title>{m['live_deck.title']()} · SCUTOONS</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<!-- #478: stage only (deck-stage-only.svelte) – the deck covers the reader's menu and logo -->
<div class="deck" class:stage-only={page.state.deckStageOnly} data-testid="live-deck">
	{#if flash}<div class="strobe" data-testid="deck-strobe"></div>{/if}
	<div class="stage" bind:this={stage}>
		{#if !threeD && face}<img class="face" src={face} alt="" crossorigin="anonymous" />{/if}
	</div>
	<DeckStageOnly raised={!idle} />
	{#if !idle}
		{#if sticksOn}
			<DeckSticks labels={ctl.labels} ondown={(s) => ctl.hands?.stickDown(s)} onmove={(s, x, y) => ctl.hands?.stickMove(s, x, y)} onup={(s) => ctl.hands?.stickUp(s)} />
		{/if}
		<!-- #460: the sound needs a tap; the same round Play as <EngineStage gate> (#298) -->
		{#if !started}
			<div class="play sc-tool">
				<button type="button" class="sc-round" title={starting ? m['live_deck.starting']() : m['live_deck.start']()} aria-label={m['live_deck.start']()} disabled={starting} data-testid="deck-start" onclick={start}>
					<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v14l11-7z" fill="currentColor" /></svg>
				</button>
			</div>
		{/if}
		<div class="bottom">
			<DeckPanel
				{values}
				{params}
				{started}
				{stats}
				{status}
				alert={audioAlert}
				sticks={ctl.line}
				selected={ctl.knob}
				learn={ctl.learn}
				learning={ctl.learning}
				midi={ctl.midiStatus}
				mapped={ctl.mapped}
				turns={ctl.hands?.c ?? null}
				onset={(p, v) => engine?.deck?.setParam(p, v)}
				onpick={ctl.pick}
				onlearn={ctl.toggleLearn}
				{sticksOn}
				onsticks={toggleSticks}
				{quality}
				onquality={setQuality}
				oncopy={copyStats}
				onsettings={() => {
					keepPresets();
					sheetOpen = true;
				}}
				ontouch={(on) => engine?.deck && (engine.deck.touch = on)}
				{clock}
				{clockModes}
				{strobe}
				onclock={(c) => engine?.deck?.clockMode(c)}
				onclick={() => engine?.deck?.clockKey('click')}
				onstrobe={() => (strobe = !strobe)}
				onmidiout={() => engine?.deck?.clockOut(!clock?.midiOut)}
			/>
		</div>
		{#if presets && quality && qualityOptions}
			<DeckQualitySheet bind:open={sheetOpen} {presets} {quality} options={qualityOptions} screen={screenInfo} text={presetsText} onchange={setPresets} ontext={setPresetsText} />
		{/if}
	{/if}
</div>

<style>
	.deck {
		position: fixed;
		inset: 0;
		background: var(--ink);
		color: var(--paper);
		overflow: hidden;
	}
	/* over the reader's menu (1003) and logo (1004, main-menu.svelte) */
	.deck.stage-only {
		z-index: 1005;
	}
	.stage {
		position: absolute;
		inset: 0;
	}
	/* #321: the strobe on the beat – over everything, never in the way of a finger */
	.strobe {
		position: absolute;
		inset: 0;
		z-index: 10;
		background: var(--paper);
		pointer-events: none;
	}
	.face {
		position: absolute;
		inset: 20% 15%;
		width: 70%;
		height: 60%;
		object-fit: contain;
	}
	.play {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		background: none;
	}
	.play svg {
		width: 1.5rem;
		height: 1.5rem;
	}
	.bottom {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		pointer-events: none;
	}
</style>
