<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import { onMount, untrack } from 'svelte';
	import type { CameraRequest } from '$lib/contexts/items-ui.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { SCAN_OBJECT_FPS, SCAN_OBJECT_FRAMES, SCAN_OBJECT_MIN, SCAN_OBJECT_THRESHOLD, SCAN_OBJECT_TOP } from '$lib/config.ts';
	import ItemIcon from '$lib/components/items/item-icon.svelte';
	import { decodeFrame, loadJsQR, parseScutoonsCode } from './qr.ts';
	import { cropSquare, loadClassifier, squareUnder, type Classifier } from './object-detector.ts';
	import { maxOf, scanPasses, scanTargets, scoreScan } from '@scutoons/content';
	import { localized } from '$lib/contexts/inventory.svelte.ts';
	import { findKnown, remember } from './scan-memory.ts';

	let {
		request,
		onclose,
		oncode
	}: { request: CameraRequest; onclose: () => void; oncode: (code: string) => void } = $props();

	const inventory = getInventoryContext();

	type Mode = 'qr' | 'object';
	type Status = 'starting' | 'running' | 'denied' | 'unsupported' | 'insecure' | 'not_found' | 'error';

	// one mode per session (request is fixed per overlay): the cucumber camera only detects cucumbers, no QR decoding
	const mode: Mode = untrack(() => (request.object ? 'object' : 'qr'));
	let status = $state<Status>('starting');
	let video = $state<HTMLVideoElement>();
	/** the rangefinder's ghost image (object cameras): the same stream, shifted inside the focus patch */
	let ghost = $state<HTMLVideoElement>();
	/** the focus patch in camera-overlay coordinates (CSS inset) – the ghost image is clipped to it */
	let patchClip = $state('inset(50%)');
	let canvas = $state<HTMLCanvasElement>();
	/** the aiming square: the object camera classifies exactly the part of the frame under it */
	let aim = $state<HTMLDivElement>();
	let stream: MediaStream | null = null;

	let message = $state<string | null>(null);
	let messageTimer: ReturnType<typeof setTimeout> | undefined;

	// object detection
	let modelState = $state<'idle' | 'loading' | 'ready' | 'failed'>('idle');
	/** 0..1 of a passing frame (1 = this frame passes) – the meter */
	let confidence = $state(0);
	let streak = 0;
	let cooldownUntil = $state(0);
	let now = $state(Date.now());
	let celebrate = $state(0);

	const skill = $derived(request.object);
	const rule = $derived({ threshold: skill?.threshold ?? SCAN_OBJECT_THRESHOLD, top: SCAN_OBJECT_TOP, min: SCAN_OBJECT_MIN });
	const cooldownLeft = $derived(Math.max(0, Math.ceil((cooldownUntil - now) / 1000)));
	/** what the last hit was recognised as and which item it became (game text, never an analysis) */
	let lastHit = $state<{ grants: string; object: string } | null>(null);
	/** "I know that one!": the same thing scanned again (scan-memory.ts) – the reader is told it is cheating */
	let known = $state(false);
	/** the server's daily limit (429 daily_limit) or the collection limit (409 max_reached) was reached */
	let enough = $state<'daily_limit' | 'max_reached' | null>(null);
	/** the item the last grant attempt was for (the collection-limit text) */
	let lastScanned = $state('');
	/** a camera that finds substances (testo / Check It camera) shows the drug-checking notice every time it opens */
	const substanceCamera = $derived(!!skill && scanTargets(skill).some((t) => inventory.item(t.grants)?.category === 'substance'));
	/** checkit! – the real, anonymous drug-checking service of Suchthilfe Wien (official site checked 2026-10-01) */
	const CHECKIT_URL = 'https://checkit.wien';
	/** the cucumber camera keeps its cucumber texts; every other object camera (testo, Check It) uses its item name */
	const cucumberCamera = $derived(!!skill && scanTargets(skill).every((t) => t.grants === 'cucumber'));
	const objectTitle = $derived(cucumberCamera || !request.entry ? m['camera.title_object']() : inventory.nameOf({ item_id: request.entry.item_id, variant: null }));

	function flash(text: string) {
		message = text;
		clearTimeout(messageTimer);
		messageTimer = setTimeout(() => (message = null), 2200);
	}

	async function start() {
		status = 'starting';
		if (typeof window !== 'undefined' && !window.isSecureContext) {
			status = 'insecure';
			return;
		}
		if (!navigator.mediaDevices?.getUserMedia) {
			status = 'unsupported';
			return;
		}
		try {
			stream = await navigator.mediaDevices.getUserMedia({
				video: { facingMode: { ideal: 'environment' }, width: { ideal: 1920 }, height: { ideal: 1080 } },
				audio: false
			});
			if (!video) return;
			video.srcObject = stream;
			await video.play();
			if (ghost) {
				ghost.srcObject = stream;
				ghost.play().catch(() => {});
			}
			status = 'running';
		} catch (error: any) {
			const name = error?.name;
			status =
				name === 'NotAllowedError' || name === 'SecurityError'
					? 'denied'
					: name === 'NotFoundError' || name === 'OverconstrainedError'
						? 'not_found'
						: 'error';
			console.warn('Camera failed:', error);
		}
	}

	function stop() {
		stream?.getTracks().forEach((t) => t.stop());
		stream = null;
	}

	// ---------- QR loop (~8 fps) ----------
	async function qrLoop(signal: { stopped: boolean }) {
		const jsQR = await loadJsQR();
		let lastText = '';
		while (!signal.stopped) {
			if (status === 'running' && video && canvas && video.readyState >= 2) {
				const text = decodeFrame(jsQR, video, canvas);
				if (text) {
					const code = parseScutoonsCode(text);
					if (code) {
						oncode(code);
						return;
					}
					if (text !== lastText) flash(m['camera.not_scutoons']());
					lastText = text;
				}
			}
			await new Promise((r) => setTimeout(r, 125));
		}
	}

	// ---------- object loop (SCAN_OBJECT_FPS) ----------
	async function objectLoop(signal: { stopped: boolean }) {
		if (!skill) return;
		let classifier: Classifier;
		modelState = 'loading';
		try {
			classifier = await loadClassifier();
			modelState = 'ready';
		} catch (error) {
			console.warn('Classifier failed to load', error);
			modelState = 'failed';
			return;
		}
		while (!signal.stopped) {
			now = Date.now();
			const square = status === 'running' && video && video.readyState >= 2 && aim ? squareUnder(video, aim) : null;
			if (video && square) {
				try {
					// only the frame under the aiming square (owner phone test 2026-10-02: the square scored up to 0.45 where the
					// whole frame and a 1.6× wider square stayed at 0.01–0.08)
					const predictions = await classifier.classify(cropSquare(video, square), 10);
					const hit = scoreScan(predictions, skill);
					if (import.meta.env.DEV) console.debug('scan_object', hit ? `${hit.label} ${hit.score.toFixed(2)} #${hit.rank}` : '-', '|', predictions.slice(0, 5).map((p) => `${p.className} ${p.probability.toFixed(2)}`).join(' | '));
					const passes = scanPasses(hit, rule);
					confidence = passes ? 1 : Math.min(0.9, (hit?.score ?? 0) / rule.threshold);
					streak = passes ? streak + 1 : 0;
					if (hit && passes && streak >= SCAN_OBJECT_FRAMES && now >= cooldownUntil && !enough) {
						streak = 0;
						cooldownUntil = Date.now() + (skill.cooldown_s ?? 10) * 1000;
						const fingerprint = await classifier.embed(cropSquare(video, square, 1));
						if (findKnown(hit.target.grants, fingerprint)) {
							known = true;
						} else {
							lastScanned = hit.target.grants;
							const result = await inventory.grant(hit.target.grants, 'scan', 1, `scan:${hit.label}`);
							if (result.ok) {
								remember(hit.target.grants, fingerprint);
								known = false;
								lastHit = { grants: hit.target.grants, object: localized(hit.target.name) || hit.label };
								celebrate++;
							} else if (result.error === 'daily_limit' || result.error === 'max_reached') {
								enough = result.error;
							}
						}
					}
				} catch (error) {
					console.warn('Classification failed', error);
				}
			}
			await new Promise((r) => setTimeout(r, 1000 / SCAN_OBJECT_FPS));
		}
	}

	$effect(() => {
		const signal = { stopped: false };
		if (mode === 'qr') qrLoop(signal);
		if (mode === 'object' && skill) objectLoop(signal);
		return () => (signal.stopped = true);
	});

	function measurePatch() {
		const host = aim?.closest('.camera')?.getBoundingClientRect();
		const box = aim?.getBoundingClientRect();
		if (!host || !box) return;
		patchClip = `inset(${box.top - host.top}px ${host.right - box.right}px ${host.bottom - box.bottom}px ${box.left - host.left}px)`;
	}

	$effect(() => {
		if (!aim) return;
		measurePatch();
		const observer = new ResizeObserver(measurePatch);
		observer.observe(aim);
		window.addEventListener('resize', measurePatch);
		return () => (observer.disconnect(), window.removeEventListener('resize', measurePatch));
	});

	/**
	 * Messsucher (owner 2026-10-02, design A + "sharp as focus"): the picture is soft and snaps sharp, and the ghost
	 * image in the focus patch slides onto the real one, as the camera becomes sure it sees the object.
	 */
	const blur = $derived(mode === 'object' ? Math.round((1 - confidence) * 60) / 10 : 0);
	const ghostShift = $derived(Math.round((1 - confidence) * 28));
	const segments = $derived(Math.round(confidence * 5));

	onMount(() => {
		start();
		const onKey = (e: KeyboardEvent) => e.key === 'Escape' && onclose();
		window.addEventListener('keydown', onKey);
		return () => {
			stop();
			clearTimeout(messageTimer);
			window.removeEventListener('keydown', onKey);
		};
	});

	const errorText = $derived(
		{
			denied: m['camera.denied'](),
			unsupported: m['camera.unsupported'](),
			insecure: m['camera.insecure'](),
			not_found: m['camera.not_found'](),
			error: m['camera.error']()
		}[status as string] ?? null
	);
</script>

<div class="camera" class:with-notice={substanceCamera} class:object={mode === 'object'} class:cucumber={cucumberCamera} role="dialog" aria-modal="true" aria-label={mode === 'qr' ? m['camera.title_qr']() : objectTitle}>
	<!-- svelte-ignore a11y_media_has_caption -->
	<video bind:this={video} playsinline muted autoplay class:hidden={status !== 'running'} style:filter={blur ? `blur(${blur}px)` : null}></video>
	{#if mode === 'object'}
		<!-- svelte-ignore a11y_media_has_caption -->
		<div class="ghost" style:clip-path={patchClip} aria-hidden="true" class:hidden={status !== 'running'}>
			<video bind:this={ghost} playsinline muted autoplay style:transform="translateX({ghostShift}px)"></video>
		</div>
	{/if}
	<canvas bind:this={canvas} hidden></canvas>

	<!-- 80s HUD (owner 2026-10-02, drafts 1 + 2): no bars, thin lines over the picture, 1rem mono everywhere -->
	<div class="scanlines" aria-hidden="true"></div>
	<header>
		<h2>{mode === 'qr' ? m['camera.title_qr']() : objectTitle}</h2>
		<button class="close" onclick={onclose} aria-label={m['items.close']()}>
			<!-- the one close cross (DESIGN.md "Close cross"), 1px like the HUD's thin lines -->
			<CloseIcon size="1rem" stroke={1} />
		</button>
		{#if substanceCamera}
			<!-- mandatory on every open (owner 2026-10-01): a game camera, and where real drug checking is -->
			<p class="game-notice" data-testid="camera-notice">
				{m['camera.game_notice']()}
				<a href={CHECKIT_URL} target="_blank" rel="noopener noreferrer">{m['camera.checkit']()}</a>
			</p>
		{/if}
	</header>

	{#if status === 'running'}
		<div class="frame-lines" aria-hidden="true"></div>
		<div class="ticks" aria-hidden="true"></div>
		{#if mode === 'qr'}
			<div class="viewfinder" aria-hidden="true"></div>
		{:else}
			<div class="viewfinder patch" class:sharp={confidence >= 1} bind:this={aim} aria-hidden="true"></div>
		{/if}
		{#if known}
			<div class="known" role="alert" data-testid="camera-known">
				<strong>{m['camera.known']()}</strong>
				<p>{cucumberCamera ? m['camera.known_cheat']() : m['camera.known_cheat_any']()}</p>
				<button onclick={() => (known = false)} data-testid="camera-known-ok">{m['camera.known_ok']()}</button>
			</div>
		{/if}
		{#key celebrate}
			{#if celebrate > 0 && skill && lastHit}
				<div class="celebrate" aria-live="polite" data-testid="camera-hit" data-grants={lastHit.grants}>
					<div class="burst"></div>
					<div class="icon">
						<ItemIcon srcs={[inventory.item(lastHit.grants)?.icon ?? '']} name={inventory.nameOf({ item_id: lastHit.grants, variant: null })} />
					</div>
					<strong>{scanTargets(skill).length > 1 ? m['camera.recognised']({ object: lastHit.object, item: inventory.nameOf({ item_id: lastHit.grants, variant: null }) }) : cucumberCamera ? m['camera.detected']() : `${inventory.nameOf({ item_id: lastHit.grants, variant: null })}!`}</strong>
				</div>
			{/if}
		{/key}
	{:else if status === 'starting'}
		<p class="center">{m['common.loading']()} {m['camera.starting']()}</p>
	{:else}
		<div class="center error">
			<p>{errorText}</p>
			{#if status === 'denied' || status === 'error' || status === 'not_found'}
				<button onclick={start}>{m['camera.retry']()}</button>
			{/if}
		</div>
	{/if}

	<footer>
		{#if message}
			<p class="toast">{message}</p>
		{/if}
		{#if mode === 'object' && modelState === 'ready'}
			<div class="meter" aria-hidden="true" data-testid="camera-meter" data-level={segments}>
				{#each [0, 1, 2, 3, 4] as i (i)}<span class:on={i < segments}></span>{/each}
			</div>
		{/if}
		{#if mode === 'object'}
			{#if modelState === 'loading'}
				<p>{m['camera.loading_model']()}</p>
			{:else if modelState === 'failed'}
				<p>{m['camera.model_failed']()}</p>
			{:else if enough === 'daily_limit'}
				<p data-testid="camera-daily-limit">{cucumberCamera ? m['camera.daily_limit']() : m['camera.daily_limit_any']()}</p>
			{:else if enough === 'max_reached'}
				<p data-testid="camera-max-reached">{cucumberCamera ? m['camera.max_reached']({ max: maxOf(inventory.item(lastScanned)) }) : m['camera.max_reached_any']({ max: maxOf(inventory.item(lastScanned)) })}</p>
			{:else if cooldownLeft > 0}
				<p>{cucumberCamera ? m['camera.cooldown']({ seconds: cooldownLeft }) : m['camera.cooldown_any']({ seconds: cooldownLeft })}</p>
			{:else}
				<p>{cucumberCamera ? m['camera.hint_object']() : m['camera.hint_any']()}</p>
			{/if}
		{:else}
			<p>{m['camera.hint_qr']()}</p>
		{/if}
	</footer>
</div>

<style>
	.camera {
		position: fixed;
		inset: 0;
		z-index: 2000;
		background: black;
		color: var(--finder);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		font-family: var(--font-mono);
		font-size: 1rem;
		line-height: 1.4;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		/* readable on a bright picture, with a little phosphor glow */
		text-shadow:
			0 0 0.125rem rgb(0 0 0 / 0.9),
			0 0 0.5rem color-mix(in srgb, var(--finder) 45%, transparent);
	}
	video {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	video.hidden {
		visibility: hidden;
	}
	.scanlines {
		position: absolute;
		inset: 0;
		z-index: 1;
		pointer-events: none;
		background: repeating-linear-gradient(to bottom, transparent 0 3px, rgb(0 0 0 / 0.16) 3px 4px);
	}
	header,
	footer {
		position: relative;
		z-index: 3;
		padding: 1rem 1.25rem;
	}
	header {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		justify-content: space-between;
		gap: 1rem;
	}
	h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 400;
		letter-spacing: inherit;
	}
	.close {
		flex: none;
		display: grid;
		place-items: center;
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border: var(--border-width-regular) solid var(--finder);
		border-radius: 0;
		background: transparent;
		color: var(--finder);
		box-shadow: 0 0 0.5rem color-mix(in srgb, var(--finder) 35%, transparent);
	}
	.close:hover,
	.close:focus-visible {
		background: color-mix(in srgb, var(--finder) 20%, transparent);
		outline: none;
	}
	.game-notice {
		flex-basis: 100%;
		margin: 0;
		font-size: 1rem;
		text-transform: none;
		letter-spacing: 0;
	}
	.game-notice a {
		color: inherit;
	}
	footer {
		margin-top: auto;
	}
	footer p {
		margin: 0;
		font-size: 1rem;
	}
	.toast {
		margin-bottom: 0.5rem !important;
		padding: 0.25rem 0.5rem;
		border: 1px solid var(--finder);
		background: rgb(0 0 0 / 0.45);
	}
	.center {
		position: relative;
		z-index: 3;
		margin: auto;
		padding: 1rem;
		max-width: 24rem;
		text-align: center;
	}
	.error p {
		margin: 0 0 1rem;
	}
	.error button,
	.known button {
		border: var(--border-width-regular) solid var(--finder);
		background: transparent;
		color: var(--finder);
		font: inherit;
		text-transform: inherit;
		letter-spacing: inherit;
		padding: 0 1rem;
	}
	.camera {
		/* the camera's markings: the cucumber camera is green, every other camera white (owner 2026-10-02) */
		--finder: white;
	}
	.camera.cucumber {
		--finder: var(--acid-green);
	}
	video {
		transition: filter 0.4s ease;
	}
	.ghost {
		position: absolute;
		inset: 0;
		z-index: 1;
		pointer-events: none;
		mix-blend-mode: screen;
		opacity: 0.55;
	}
	.ghost.hidden {
		visibility: hidden;
	}
	.ghost video {
		transition: transform 0.4s ease;
	}
	/* bright-line frame of a rangefinder: four corners inset from the picture */
	.frame-lines {
		position: absolute;
		z-index: 1;
		inset: 4rem 1.25rem 6rem;
		pointer-events: none;
		--c: var(--finder);
		background:
			linear-gradient(var(--c), var(--c)) top left / 1.5rem 1px no-repeat,
			linear-gradient(var(--c), var(--c)) top left / 1px 1.5rem no-repeat,
			linear-gradient(var(--c), var(--c)) top right / 1.5rem 1px no-repeat,
			linear-gradient(var(--c), var(--c)) top right / 1px 1.5rem no-repeat,
			linear-gradient(var(--c), var(--c)) bottom left / 1.5rem 1px no-repeat,
			linear-gradient(var(--c), var(--c)) bottom left / 1px 1.5rem no-repeat,
			linear-gradient(var(--c), var(--c)) bottom right / 1.5rem 1px no-repeat,
			linear-gradient(var(--c), var(--c)) bottom right / 1px 1.5rem no-repeat;
	}
	.with-notice .frame-lines,
	.with-notice .ticks {
		top: 12rem;
	}
	/* range-finder ticks down both edges of the frame */
	.ticks {
		position: absolute;
		z-index: 1;
		inset: 6rem 1.25rem 8rem;
		pointer-events: none;
		--c: var(--finder);
		background:
			repeating-linear-gradient(to bottom, var(--c) 0 1px, transparent 1px 1.5rem) left / 0.375rem 100% no-repeat,
			repeating-linear-gradient(to bottom, var(--c) 0 1px, transparent 1px 4.5rem) left / 0.75rem 100% no-repeat,
			repeating-linear-gradient(to bottom, var(--c) 0 1px, transparent 1px 1.5rem) right / 0.375rem 100% no-repeat,
			repeating-linear-gradient(to bottom, var(--c) 0 1px, transparent 1px 4.5rem) right / 0.75rem 100% no-repeat;
		opacity: 0.8;
	}
	.viewfinder {
		position: relative;
		z-index: 2;
		margin: auto;
		width: min(65vw, 18rem);
		aspect-ratio: 1;
		border: 1px solid var(--finder);
	}
	/* crosshair with an open centre */
	.viewfinder.patch::before,
	.viewfinder.patch::after {
		content: '';
		position: absolute;
		left: 50%;
		top: 50%;
		transform: translate(-50%, -50%);
		--c: var(--finder);
	}
	.viewfinder.patch::before {
		width: 3rem;
		height: 1px;
		background: linear-gradient(to right, var(--c) 0 35%, transparent 35% 65%, var(--c) 65%);
	}
	.viewfinder.patch::after {
		width: 1px;
		height: 3rem;
		background: linear-gradient(to bottom, var(--c) 0 35%, transparent 35% 65%, var(--c) 65%);
	}
	/* the focus patch: what the camera judges; turns solid when it is sure */
	.viewfinder.patch.sharp {
		border-width: 3px;
	}
	.meter {
		display: flex;
		gap: 0.375rem;
		margin-bottom: 0.5rem;
	}
	.meter span {
		width: 1.5rem;
		height: 0.375rem;
		border: 1px solid var(--finder);
		box-shadow: 0 0 0.375rem color-mix(in srgb, var(--finder) 35%, transparent);
	}
	.meter span.on {
		background: var(--finder);
	}
	.known {
		position: absolute;
		z-index: 4;
		left: 1.25rem;
		right: 1.25rem;
		top: 50%;
		transform: translateY(-50%);
		background: rgb(0 0 0 / 0.6);
		border: 1px solid var(--finder);
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
		text-align: center;
	}
	.known strong {
		font-size: 1rem;
		font-weight: 400;
	}
	.known p {
		margin: 0;
		font-size: 1rem;
		text-transform: none;
		letter-spacing: 0;
	}
	.known button {
		align-self: center;
	}
	.celebrate {
		position: absolute;
		z-index: 2;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		pointer-events: none;
		animation: celebrate-out 0.3s ease 1.6s forwards;
	}
	.burst {
		position: absolute;
		inset: 0;
		background: var(--primary-color);
		animation: burst 0.3s ease forwards;
	}
	.celebrate .icon {
		width: 10rem;
		filter: drop-shadow(0 0 16px var(--primary-color));
		animation: pop 0.3s ease forwards;
	}
	.celebrate strong {
		position: relative;
		margin-top: 0.5rem;
		background: rgb(0 0 0 / 0.6);
		color: var(--finder);
		padding: 0.25rem 0.75rem;
		font-weight: 400;
		border: 1px solid var(--finder);
		animation: pop 0.3s ease forwards;
	}
	@keyframes burst {
		from {
			opacity: 0.8;
		}
		to {
			opacity: 0;
		}
	}
	@keyframes pop {
		0% {
			transform: scale(0.3) rotate(-12deg);
			opacity: 0;
		}
		70% {
			transform: scale(1.15) rotate(4deg);
			opacity: 1;
		}
		100% {
			transform: scale(1) rotate(0);
			opacity: 1;
		}
	}
	@keyframes celebrate-out {
		to {
			opacity: 0;
		}
	}
</style>
