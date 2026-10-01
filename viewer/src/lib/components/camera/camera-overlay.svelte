<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import type { CameraRequest } from '$lib/contexts/items-ui.svelte.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { SCAN_OBJECT_FPS, SCAN_OBJECT_FRAMES, SCAN_OBJECT_THRESHOLD } from '$lib/config.ts';
	import ItemIcon from '$lib/components/items/item-icon.svelte';
	import { decodeFrame, loadJsQR, parseScutoonsCode } from './qr.ts';
	import { centerCrop, loadClassifier, type Classifier } from './object-detector.ts';
	import { matchScan, scanTargets } from '@scutoons/content';
	import { localized } from '$lib/contexts/inventory.svelte.ts';

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
	let canvas = $state<HTMLCanvasElement>();
	let stream: MediaStream | null = null;

	let message = $state<string | null>(null);
	let messageTimer: ReturnType<typeof setTimeout> | undefined;

	// object detection
	let modelState = $state<'idle' | 'loading' | 'ready' | 'failed'>('idle');
	let confidence = $state(0);
	let streak = 0;
	let cooldownUntil = $state(0);
	let now = $state(Date.now());
	let celebrate = $state(0);

	const skill = $derived(request.object);
	const threshold = $derived(skill?.threshold ?? SCAN_OBJECT_THRESHOLD);
	const cooldownLeft = $derived(Math.max(0, Math.ceil((cooldownUntil - now) / 1000)));
	/** what the last hit was recognised as and which item it became (game text, never an analysis) */
	let lastHit = $state<{ grants: string; object: string } | null>(null);
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
				video: { facingMode: { ideal: 'environment' }, width: { ideal: 1280 }, height: { ideal: 720 } },
				audio: false
			});
			if (!video) return;
			video.srcObject = stream;
			await video.play();
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
			if (status === 'running' && video && video.readyState >= 2) {
				try {
					const predictions = await classifier.classify(centerCrop(video));
					if (import.meta.env.DEV) console.debug('scan_object', predictions.map((p) => `${p.className} ${p.probability.toFixed(2)}`).join(' | '));
					const hit = matchScan(predictions, skill);
					confidence = hit?.probability ?? 0;
					streak = hit && hit.probability >= threshold ? streak + 1 : 0;
					if (hit && streak >= SCAN_OBJECT_FRAMES && now >= cooldownUntil) {
						streak = 0;
						cooldownUntil = Date.now() + (skill.cooldown_s ?? 10) * 1000;
						lastHit = { grants: hit.target.grants, object: localized(hit.target.name) || hit.label };
						celebrate++;
						await inventory.grant(hit.target.grants, 'scan', 1, `scan:${hit.label}`);
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

<div class="camera" class:with-notice={substanceCamera} role="dialog" aria-modal="true" aria-label={mode === 'qr' ? m['camera.title_qr']() : objectTitle}>
	<!-- svelte-ignore a11y_media_has_caption -->
	<video bind:this={video} playsinline muted autoplay class:hidden={status !== 'running'}></video>
	<canvas bind:this={canvas} hidden></canvas>

	<header>
		<h2>{mode === 'qr' ? m['camera.title_qr']() : objectTitle}</h2>
		<button class="close" onclick={onclose} aria-label={m['items.close']()}>×</button>
		{#if substanceCamera}
			<!-- mandatory on every open (owner 2026-10-01): a game camera, and where real drug checking is -->
			<p class="game-notice" data-testid="camera-notice">
				{m['camera.game_notice']()}
				<a href={CHECKIT_URL} target="_blank" rel="noopener noreferrer">{m['camera.checkit']()}</a>
			</p>
		{/if}
	</header>

	{#if status === 'running'}
		{#if mode === 'qr'}
			<div class="viewfinder" aria-hidden="true"></div>
		{:else}
			<div class="meter" aria-hidden="true">
				<div class="fill" style="width:{Math.round(confidence * 100)}%"></div>
				<div class="mark" style="left:{threshold * 100}%"></div>
			</div>
		{/if}
		{#key celebrate}
			{#if celebrate > 0 && skill && lastHit}
				<div class="celebrate" aria-live="polite" data-testid="camera-hit" data-grants={lastHit.grants}>
					<div class="burst"></div>
					<div class="icon">
						<ItemIcon srcs={[inventory.item(lastHit.grants)?.icon ?? '']} name={inventory.nameOf({ item_id: lastHit.grants, variant: null })} />
					</div>
					<strong>{scanTargets(skill).length > 1 ? m['camera.recognised']({ object: lastHit.object, item: inventory.nameOf({ item_id: lastHit.grants, variant: null }) }) : m['camera.detected']()}</strong>
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
		{#if mode === 'object'}
			{#if modelState === 'loading'}
				<p>{m['camera.loading_model']()}</p>
			{:else if modelState === 'failed'}
				<p>{m['camera.model_failed']()}</p>
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
		color: white;
		display: flex;
		flex-direction: column;
		overflow: hidden;
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
	header,
	footer {
		position: relative;
		z-index: 1;
		background: black;
		padding: 0.75rem 1rem;
	}
	.game-notice {
		flex-basis: 100%;
		margin: 0.25rem 0 0;
		font-size: 0.6875rem;
		line-height: 1.3;
		color: #d0d0d0;
	}
	.game-notice a {
		color: white;
	}
	header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		border-bottom: 2px solid white;
	}
	h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}
	.close {
		width: 2rem;
		height: 2rem;
		padding: 0;
		border-radius: 50%;
		background: white;
		color: black;
		border: 2px solid black;
		font-size: 1.25rem;
		line-height: 1;
		flex: none;
	}
	.close:hover {
		background: var(--primary-color);
	}
	footer {
		margin-top: auto;
		border-top: 2px solid white;
	}
	footer p {
		margin: 0 0 0.5rem;
		font-size: 0.75rem;
		line-height: 1.3;
	}
	.toast {
		background: white;
		color: black;
		padding: 0.5rem;
		font-size: 1rem !important;
	}
	.center {
		position: relative;
		z-index: 1;
		margin: auto;
		padding: 1rem;
		max-width: 24rem;
		text-align: center;
	}
	.error p {
		margin: 0 0 1rem;
	}
	.error button {
		background: white;
		color: black;
	}
	.viewfinder {
		position: relative;
		z-index: 1;
		margin: auto;
		width: min(65vw, 18rem);
		aspect-ratio: 1;
		border: 2px solid white;
		box-shadow: 0 0 0 100vmax rgba(0, 0, 0, 0.35);
		filter: drop-shadow(0 0 8px var(--primary-color));
	}
	.meter {
		position: absolute;
		z-index: 1;
		left: 1rem;
		right: 1rem;
		top: 4.5rem;
		height: 0.5rem;
		border: 2px solid white;
		background: rgba(0, 0, 0, 0.4);
	}
	.with-notice .meter {
		top: 7rem;
	}
	.meter .fill {
		height: 100%;
		background: var(--primary-color);
		transition: width 0.3s linear;
	}
	.meter .mark {
		position: absolute;
		top: -0.25rem;
		bottom: -0.25rem;
		width: 2px;
		background: white;
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
		background: white;
		color: black;
		padding: 0.25rem 0.75rem;
		text-transform: uppercase;
		border: 2px solid black;
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
