<script>
	// The phone page of the handheld camera (/phone?code=1234 – README "Handheld camera", research/HANDHELD-CAMERA.md):
	// the shot's scene fullscreen through the shot camera, moved by the phone. WebXR immersive-ar (ARCore: position +
	// rotation) where the phone has it, its own render over the camera image (dom-overlay); else DeviceOrientation
	// (rotation) + drag for position (one finger = move, two = up / down). Set origin = the phone's pose now is the shot
	// camera's pose. Every frame the camera's pose goes to the PC (which smooths, shows and records); Record works here too.
	import * as THREE from 'three';
	import { onDestroy, onMount } from 'svelte';
	import { buildScene, seedOf, sinPass } from '../../../tools/storyboard-render/core.js';
	import { connect } from './lib/handheld/link.js';
	import { makeOrigin, mapPose, orientationQuat, yawOf } from './lib/handheld/track.js';

	const code = new URLSearchParams(location.search).get('code') ?? '';
	let root = $state(), stage = $state();
	let status = $state(code ? 'Waiting for the PC – open a shot\'s Handheld panel there …' : 'No code – open the link from the PC\'s Handheld panel.');
	let shotId = $state('');
	let mode = $state(''); // 'xr' | 'orientation'
	let recording = $state(false), recFrom = 0, recShown = $state(0);
	let tracking = $state(true);
	let styled = $state(true);
	const xrOk = $state({ v: false });
	let link, renderer, pass, built = null, specKey = '', scale = 1;
	let rest = null; // the shot camera as built { p, q }
	let raw = null; // the phone's pose { position, quaternion } (its own world)
	let phoneOrigin = null, origin = null;
	let session = null, refSpace = null, glx = null, raf = 0, wake = null;
	const drag = new THREE.Vector3();
	let size = { w: 0, h: 0 };

	onMount(async () => {
		renderer = new THREE.WebGLRenderer({ antialias: false });
		stage.appendChild(renderer.domElement);
		pass = sinPass(renderer);
		xrOk.v = !!(await navigator.xr?.isSessionSupported?.('immersive-ar').catch(() => false));
		if (!code) return;
		link = connect(code, 'phone', onMessage);
		addEventListener('resize', fit);
	});
	onDestroy(() => { cancelAnimationFrame(raf); session?.end().catch(() => {}); link?.close(); built?.dispose(); pass?.dispose(); renderer?.dispose(); removeEventListener('deviceorientation', onOrient); wake?.release?.(); });

	async function onMessage(m) {
		if (m.type === 'peer') sendState();
		else if (m.type === 'left') status = 'The PC left – reopen the link from its Handheld panel.';
		else if (m.type === 'state') {
			if (m.recording && !recording) recFrom = performance.now();
			recording = !!m.recording;
			scale = m.scale ?? 1;
			if (m.on === false) status = 'The handheld camera is off for this shot (PC).';
		} else if (m.type === 'origin') setOrigin();
		else if (m.type === 'spec') await build(m.shot, m.spec);
	}
	const sendState = () => link?.send('state', { mode, tracking, origin: !!phoneOrigin });

	async function build(id, spec) {
		const key = JSON.stringify(spec);
		if (key === specKey) return;
		specKey = key;
		const was = shotId;
		shotId = id;
		status = `Building ${id} …`;
		const next = await buildScene(spec, { shadowSize: 1024 });
		if (specKey !== key) { next.dispose(); return; }
		built?.dispose();
		built = { ...next, spec };
		rest = { p: next.camera.position.toArray(), q: next.camera.quaternion.toArray() };
		if (was !== id) phoneOrigin = null; // another shot: start from its camera again
		remakeOrigin();
		status = '';
		fit();
		if (!mode) draw();
	}
	function remakeOrigin() {
		origin = phoneOrigin && rest ? makeOrigin(phoneOrigin.position, phoneOrigin.quaternion, rest.p, rest.q) : null;
	}
	/** the phone's pose now = the shot camera's pose now */
	function setOrigin() {
		if (!raw) return;
		phoneOrigin = { position: [...raw.position], quaternion: [...raw.quaternion] };
		remakeOrigin();
		sendState();
	}

	// the frame: the shot's aspect, as large as the screen allows; the render at the device's pixels (≤ 1.5×)
	function fit() {
		if (!built || !root) return;
		const sw = root.clientWidth, sh = root.clientHeight, a = built.spec.outW / built.spec.outH;
		const cw = Math.min(sw, sh * a), ch = cw / a;
		const k = Math.min(window.devicePixelRatio || 1, 1.5, 1600 / Math.max(cw, ch)); // ≤ 1.5×: the look's passes stay fluid on a phone
		size = { w: Math.round(cw * k), h: Math.round(ch * k) };
		const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16; // rem, never px (DESIGN)
		renderer.domElement.style.width = `${cw / rem}rem`;
		renderer.domElement.style.height = `${ch / rem}rem`;
		built.camera.aspect = a;
		built.camera.updateProjectionMatrix();
	}
	function draw() {
		if (!built || !size.w) return;
		const cam = built.camera;
		if (styled) pass.render(built.scene, cam, size.w, size.h, { sky: built.skyWhite ? 1 : 0, accent: built.spec.accent ?? '#c00000', K: size.w / built.spec.outW, seed: seedOf(built.spec.id) });
		else { renderer.setSize(size.w, size.h, false); renderer.setRenderTarget(null); renderer.setClearColor(built.skyWhite ? 0xffffff : 0x000000, 1); renderer.render(built.scene, cam); }
	}
	// a pose: the phone's pose → the camera → the PC, at the sensor's rate (XR: every XR frame; orientation: every event)
	// – the render follows at the screen's pace, so a slow render never thins out the recorded track
	function sendPose(t) {
		if (!built || !raw) return;
		if (!phoneOrigin) setOrigin(); // the first pose starts at the shot camera
		const pose = mapPose(origin, raw.position, raw.quaternion, scale);
		const cam = built.camera;
		cam.position.fromArray(pose.position);
		cam.quaternion.fromArray(pose.quaternion);
		cam.updateMatrixWorld();
		link?.send('pose', { shot: shotId, t, position: pose.position, quaternion: pose.quaternion, fov: cam.fov });
	}
	function frame() {
		draw();
		if (recording) recShown = (performance.now() - recFrom) / 1000;
	}

	// --- WebXR: ARCore's 6DoF pose; our render over the camera image (dom-overlay) ----------------------------------
	async function startXR() {
		try {
			session = await navigator.xr.requestSession('immersive-ar', { requiredFeatures: ['local'], optionalFeatures: ['dom-overlay'], domOverlay: { root } });
		} catch (e) {
			status = `WebXR did not start (${e.message}) – rotation only.`;
			return startOrientation();
		}
		if (!session.domOverlayState) {
			await session.end();
			status = 'This browser has no WebXR dom-overlay – rotation only.';
			return startOrientation();
		}
		glx = document.createElement('canvas').getContext('webgl', { xrCompatible: true });
		session.updateRenderState({ baseLayer: new XRWebGLLayer(session, glx) });
		refSpace = await session.requestReferenceSpace('local');
		mode = 'xr';
		session.addEventListener('end', () => { session = null; mode = ''; sendState(); });
		session.requestAnimationFrame(onXR);
		keepAwake();
		sendState();
	}
	function onXR(time, xrFrame) {
		if (!session) return;
		session.requestAnimationFrame(onXR);
		const layer = session.renderState.baseLayer;
		glx.bindFramebuffer(glx.FRAMEBUFFER, layer.framebuffer);
		glx.clearColor(0, 0, 0, 1);
		glx.clear(glx.COLOR_BUFFER_BIT);
		const pose = xrFrame.getViewerPose(refSpace);
		const ok = !!pose && !pose.emulatedPosition;
		if (ok !== tracking) { tracking = ok; sendState(); }
		if (pose) {
			const { position: p, orientation: o } = pose.transform;
			raw = { position: [p.x, p.y, p.z], quaternion: [o.x, o.y, o.z, o.w] };
			sendPose(time);
		}
		frame();
	}

	// --- the fallback: DeviceOrientation (rotation) + drag (position) ------------------------------------------------
	let orient = null;
	function onOrient(e) {
		if (e.alpha == null) return;
		orient = orientationQuat(e.alpha, e.beta ?? 0, e.gamma ?? 0, screen.orientation?.angle ?? 0);
		raw = { position: drag.toArray(), quaternion: orient };
		sendPose(performance.now());
	}
	async function startOrientation() {
		try { if (typeof DeviceOrientationEvent?.requestPermission === 'function') await DeviceOrientationEvent.requestPermission(); } catch {}
		addEventListener('deviceorientation', onOrient);
		mode = 'orientation';
		root.requestFullscreen?.().catch(() => {});
		keepAwake();
		sendState();
		const loop = () => {
			raf = requestAnimationFrame(loop);
			frame();
		};
		raf = requestAnimationFrame(loop);
	}
	const pointers = new Map();
	function onDown(e) { if (mode !== 'orientation') return; pointers.set(e.pointerId, { x: e.clientX, y: e.clientY }); stage.setPointerCapture?.(e.pointerId); }
	function onMove(e) {
		const last = pointers.get(e.pointerId);
		if (!last || !orient) return;
		const dx = e.clientX - last.x, dy = e.clientY - last.y;
		pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
		const k = 0.01 / pointers.size; // m per px (two fingers: each moves half)
		if (pointers.size >= 2) drag.y -= dy * k;
		else {
			const yaw = yawOf(new THREE.Quaternion().fromArray(orient));
			const fwd = new THREE.Vector3(-Math.sin(yaw), 0, -Math.cos(yaw)), right = new THREE.Vector3(Math.cos(yaw), 0, -Math.sin(yaw));
			drag.addScaledVector(right, dx * k).addScaledVector(fwd, -dy * k);
		}
	}
	const onUp = (e) => pointers.delete(e.pointerId);

	// for checks on the desktop (dev only): a simulated phone fires deviceorientation events and draws by hand (a hidden
	// test tab gets no animation frames)
	if (import.meta.env.DEV) window.__phone = { draw: frame };
	async function keepAwake() { try { wake = await navigator.wakeLock?.request('screen'); } catch {} }
	const rec = () => link?.send('rec', { on: !recording });
</script>

<div class="phone" bind:this={root}>
	<div class="stage" role="application" aria-label="The shot – drag to move (rotation mode)" bind:this={stage} onpointerdown={onDown} onpointermove={onMove} onpointerup={onUp} onpointercancel={onUp}></div>
	<header class="bar">
		<span>{shotId || 'Handheld'} · {code}</span>
		{#if mode}<span>{mode === 'xr' ? (tracking ? '6DoF' : 'tracking lost') : 'rotation + drag'}</span>{/if}
		{#if recording}<span class="rec">● {recShown.toFixed(1)} s</span>{/if}
	</header>
	{#if status}<p class="status" role="status">{status}</p>{/if}
	<footer class="bar bottom">
		{#if !mode}
			{#if xrOk.v}<button type="button" onclick={startXR} disabled={!shotId}>Start (AR)</button>{/if}
			<button type="button" class:secondary={xrOk.v} onclick={startOrientation} disabled={!shotId}>{xrOk.v ? 'Rotation only' : 'Start'}</button>
		{:else}
			<button type="button" class="secondary" onclick={() => { styled = !styled; }}>{styled ? 'Plain' : 'Look'}</button>
			<button type="button" class="secondary" onclick={setOrigin}>Set origin</button>
			<button type="button" class:rec-on={recording} onclick={rec}>{recording ? 'Stop' : 'Record'}</button>
		{/if}
	</footer>
</div>

<style>
	.phone {
		position: fixed;
		inset: 0;
		background: var(--ink);
		color: var(--paper);
		overflow: hidden;
		touch-action: none;
		user-select: none;
	}
	.stage {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.stage :global(canvas) {
		display: block;
	}
	.bar {
		position: absolute;
		left: 0;
		right: 0;
		top: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
		align-items: center;
		padding: var(--space-05) var(--space-1);
		font-size: var(--type-caption-size);
		pointer-events: none;
	}
	.bar span {
		background: var(--ink);
		padding: 0 var(--space-05);
	}
	.bottom {
		top: auto;
		bottom: 0;
		justify-content: center;
		padding-bottom: calc(var(--space-1) + env(safe-area-inset-bottom));
		pointer-events: auto;
	}
	/* three to a row on a phone: the compact side padding keeps "Set origin" on one line at 360–390 wide */
	.bottom button {
		flex: 1 1 0;
		max-width: 12rem;
		padding: 0 var(--space-05);
	}
	.rec,
	.rec-on {
		background: var(--state-error) !important;
		color: var(--paper);
	}
	/* Stop while recording: red with its own frame (the phone's primary frame is ink – buttons.css) */
	.rec-on {
		border-color: var(--state-error);
	}
	.status {
		position: absolute;
		left: var(--space-1);
		right: var(--space-1);
		top: 40%;
		margin: 0;
		text-align: center;
	}
</style>
