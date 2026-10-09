<script>
	// The 3D view (owner 2026-10-09: "can this be just a free moving camera in a 3d view since we already use threejs?"):
	// the shot's scene (the frames' own renderer, tools/storyboard-render/core.js) with the Sin City pass live. Drag =
	// orbit, right-drag / Shift = pan, wheel = dolly; the camera is the shot's camera – every move is written back as
	// shot.camera { pos, target, fov }. Click a figure → a gizmo moves it on the ground (or turns it); its place is
	// written back as the storyboard's screen position (x, y, h, turn), so frames, prompts and the draft follow.
	// The texts (the SCUTOONS presets) on top, switchable.
	import * as THREE from 'three';
	import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
	import { TransformControls } from 'three/addons/controls/TransformControls.js';
	import { onDestroy, onMount } from 'svelte';
	import { Segmented } from '@scutoons/ui';
	import { buildScene, drawDialog, figureBoxes, loadFonts, seedOf, sinPass } from '../../../../../tools/storyboard-render/core.js';
	import { specOf, studio, touched } from '../studio.svelte.js';

	const D = Math.PI / 180;
	let host = $state();
	let glHost = $state();
	let overlay = $state();
	let tool = $state('move');
	let texts = $state(true);
	let info = $state('');
	let renderer, pass, built, controls, gizmo, helperScene, spec, look;
	let pivots = [];
	const lookOf = () => ({ sky: built?.skyWhite ? 1 : 0, accent: spec?.accent ?? '#c00000', K: 1, seed: seedOf(spec?.id ?? '') });
	let size = { w: 0, h: 0, scale: 1 };
	let lastKey = '';
	let selfKey = '';
	let building = false, again = false;

	// for checks in the browser (dev only): the scene, the camera, the orbit
	if (import.meta.env.DEV) window.__studioView = { get built() { return built; }, get controls() { return controls; }, get spec() { return spec; }, get size() { return size; }, get gizmo() { return gizmo; }, get pivots() { return pivots; } };
	onMount(() => {
		renderer = new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
		renderer.domElement.classList.add('gl');
		glHost.appendChild(renderer.domElement);
		pass = sinPass(renderer);
		helperScene = new THREE.Scene();
		renderer.domElement.addEventListener('pointerdown', onPick);
		const ro = new ResizeObserver(() => { fit(); draw(); });
		ro.observe(host);
		loadFonts();
		return () => ro.disconnect();
	});
	onDestroy(() => { controls?.dispose(); gizmo?.dispose(); built?.dispose(); pass?.dispose(); renderer?.dispose(); });

	// rebuild when the shot changes – except for the changes this view wrote itself (camera, figure places)
	$effect(() => {
		const shot = studio.sel?.shot;
		if (!shot) return;
		const key = JSON.stringify($state.snapshot(shot));
		if (key === lastKey) return;
		lastKey = key;
		if (key === selfKey) return;
		rebuild();
	});
	$effect(() => { void texts; draw(); });
	$effect(() => { if (gizmo) { gizmo.setMode(tool === 'turn' ? 'rotate' : 'translate'); gizmo.showX = tool !== 'turn'; gizmo.showZ = tool !== 'turn'; gizmo.showY = tool === 'turn'; } });

	function fit() {
		if (!spec || !host) return;
		const bw = host.clientWidth - 24, bh = host.clientHeight - 24;
		const scale = Math.min(bw / spec.outW, bh / spec.outH, 2.5);
		size = { w: Math.max(32, Math.round(spec.outW * scale)), h: Math.max(32, Math.round(spec.outH * scale)), scale };
		// rem of the tools' fluid root (tools.css) – never px in CSS (owner's dogma)
		const root = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
		const rem = (px) => `${px / root}rem`;
		renderer.domElement.style.width = rem(size.w);
		renderer.domElement.style.height = rem(size.h);
		if (overlay) { overlay.width = size.w; overlay.height = size.h; overlay.style.width = rem(size.w); overlay.style.height = rem(size.h); }
		if (built?.camera.isPerspectiveCamera) { built.camera.aspect = size.w / size.h; built.camera.updateProjectionMatrix(); }
	}

	let busy = $state(false);
	async function rebuild() {
		if (building) { again = true; return; }
		building = true;
		busy = true;
		try {
			spec = specOf(1);
			const keepTarget = controls?.target.clone();
			controls?.dispose(); gizmo?.dispose(); helperScene.clear(); built?.dispose();
			// a build never blocks the next change: after 15 s it is given up (the latest state is built next)
			built = await Promise.race([buildScene(spec, { shadowSize: 2048 }), new Promise((_, no) => setTimeout(() => no(new Error('the scene took longer than 15 s – try again')), 15000))]);
			look = lookOf();
			// a pivot per figure at the centre of what the frame shows of it (a close-up's feet are below the frame): the gizmo
			// moves / turns the pivot, the figure hangs in it
			pivots = built.figs.map((m) => {
				const box = new THREE.Box3().setFromObject(m.root);
				const c = box.getCenter(new THREE.Vector3());
				const visible = new THREE.Box3();
				const p = new THREE.Vector3();
				// the centre of its part inside the camera's view, roughly: clamp the box centre into the view frustum's band
				built.camera.updateMatrixWorld();
				p.copy(c).project(built.camera);
				if (Math.abs(p.y) > 1) { p.y = Math.max(-0.8, Math.min(0.8, p.y)); p.unproject(built.camera); c.lerp(p, 1); }
				void visible;
				const pivot = new THREE.Group();
				pivot.position.copy(c);
				built.scene.add(pivot);
				pivot.attach(m.root);
				return pivot;
			});
			fit();
			const cam = built.camera;
			// the orbit's centre: the hand camera's target, else straight ahead of the shot camera at the subject's depth –
			// computed BEFORE the OrbitControls exist (its constructor already turns the camera towards its target), so the
			// view opens exactly as the frame
			const t = studio.sel.shot.camera?.target;
			const centre = new THREE.Vector3();
			if (t) centre.fromArray(t);
			else {
				const ahead = new THREE.Vector3(0, 0, -1).applyQuaternion(cam.quaternion);
				const subject = built.light.target ?? new THREE.Vector3();
				const depth = Math.max(1, subject.clone().sub(cam.position).dot(ahead));
				centre.copy(cam.position).addScaledVector(ahead, depth);
			}
			const roll = cam.rotation.z;
			controls = new OrbitControls(cam, renderer.domElement);
			controls.enableDamping = false;
			controls.target.copy(centre);
			void keepTarget;
			controls.update();
			lastCam = [...cam.position.toArray(), ...controls.target.toArray()].map((n) => n.toFixed(3)).join();
			if (!t && Math.abs(roll) > 1e-3 && cam.isPerspectiveCamera) cam.rotateZ(roll); // a dutch angle survives
			// never under the ground (a worm's-eye camera sits just above it)
			controls.addEventListener('change', () => { if (cam.position.y < 0.08) { cam.position.y = 0.08; cam.lookAt(controls.target); } draw(); });
			controls.addEventListener('end', commitCamera);
			gizmo = new TransformControls(cam, renderer.domElement);
			gizmo.size = 0.8;
			gizmo.addEventListener('dragging-changed', (e) => (controls.enabled = !e.value));
			gizmo.addEventListener('objectChange', draw);
			gizmo.addEventListener('mouseUp', commitFigures);
			helperScene.add(gizmo.getHelper());
			gizmo.setMode(tool === 'turn' ? 'rotate' : 'translate');
			if (studio.figure != null && pivots[studio.figure]) gizmo.attach(pivots[studio.figure]);
			info = '';
			draw();
		} catch (e) {
			info = `Render: ${e.message}`;
		}
		building = false;
		busy = false;
		if (again) { again = false; rebuild(); }
	}

	function draw() {
		if (!built || !size.w) return;
		pass.render(built.scene, built.camera, size.w, size.h, look);
		renderer.autoClear = false;
		renderer.render(helperScene, built.camera);
		renderer.autoClear = true;
		const g = overlay?.getContext('2d');
		if (!g) return;
		g.clearRect(0, 0, size.w, size.h);
		if (texts) drawDialog(g, spec, figureBoxes(built.figs, built.camera, size.w, size.h, 1), size.w, size.h, size.scale);
	}

	// the camera → the shot (a free camera: position, target, lens)
	let lastCam = '';
	function commitCamera() {
		const cam = built.camera;
		const shot = studio.sel.shot;
		// a click ends an orbit too – only a camera that really moved is written
		const now = [...cam.position.toArray(), ...controls.target.toArray()].map((n) => n.toFixed(3)).join();
		if (now === lastCam) return;
		lastCam = now;
		shot.camera = { pos: cam.position.toArray().map((n) => +n.toFixed(3)), target: controls.target.toArray().map((n) => +n.toFixed(3)), fov: +cam.fov.toFixed(1) };
		writeFigures(); // the figures stay where they are in the world: their screen places follow the camera
		selfKey = JSON.stringify($state.snapshot(shot));
		touched();
	}
	// the figures' world places → the storyboard's screen places (x, y across / along the shot, h in screens, turn)
	function writeFigures(only = null) {
		const shot = studio.sel.shot;
		built.scene.updateMatrixWorld(true); // the pivots moved: the figures' world boxes follow
		const boxes = figureBoxes(built.figs, built.camera, spec.w, spec.h, 1);
		for (const { index, box } of boxes) {
			if (only != null && index !== only) continue; // a figure moved by hand: only it is written
			const f = shot.figures[index];
			const m = built.figs[index];
			f.x = +(((box.x0 + box.x1) / 2) / spec.w).toFixed(3);
			f.y = +(((box.y0 + box.y1) / 2) / spec.h).toFixed(3);
			if (!f.framing || f.framing === 'full') f.h = +((box.y1 - box.y0) / 844).toFixed(3);
			const wp = m.root.getWorldPosition(new THREE.Vector3());
			const yaw = new THREE.Euler().setFromQuaternion(m.root.getWorldQuaternion(new THREE.Quaternion()), 'YXZ').y;
			const toCam = Math.atan2(built.camera.position.x - wp.x, built.camera.position.z - wp.z);
			const turn = Math.round((((yaw - toCam) / D) % 360 + 540) % 360 - 180);
			if (f.turn != null || tool === 'turn') { f.turn = turn; }
		}
	}
	function commitFigures() {
		writeFigures(studio.figure);
		selfKey = JSON.stringify($state.snapshot(studio.sel.shot));
		touched();
		draw();
	}

	// pick a figure (a click without a drag)
	let down = null;
	function onPick(e) {
		down = { x: e.clientX, y: e.clientY };
		renderer.domElement.addEventListener('pointerup', (u) => {
			if (!down || Math.hypot(u.clientX - down.x, u.clientY - down.y) > 4 || gizmo?.dragging) return;
			const r = renderer.domElement.getBoundingClientRect();
			const ndc = new THREE.Vector2(((u.clientX - r.left) / r.width) * 2 - 1, -((u.clientY - r.top) / r.height) * 2 + 1);
			const ray = new THREE.Raycaster();
			ray.setFromCamera(ndc, built.camera);
			const hit = ray.intersectObjects(built.figs.map((m) => m.root), true).find((h) => h.object.userData.fig);
			if (hit) {
				studio.figure = hit.object.userData.fig - 1;
				gizmo.attach(pivots[studio.figure]);
			} else {
				studio.figure = null;
				gizmo.detach();
			}
			draw();
		}, { once: true });
	}

	function resetCamera() {
		delete studio.sel.shot.camera;
		touched();
	}
</script>

<section class="view" aria-label="3D view">
	<div class="tools">
		<Segmented label="Figure tool" options={[{ value: 'move', label: 'Move', icon: 'move' }, { value: 'turn', label: 'Turn', icon: 'rotate-cw' }]} value={tool} onchange={(v) => (tool = v)} />
		<label class="check"><input type="checkbox" bind:checked={texts} /> Texts</label>
		<button type="button" class="secondary" onclick={resetCamera} title="Back to the camera the shot's size / angle give (removes the hand camera)">Shot camera</button>
		{#if busy}<span class="state" role="status">rendering …</span>{/if}
		{#if studio.sel?.shot.camera}<span class="state" title="The shot has its own camera from the 3D view – 'Shot camera' goes back to size + angle">hand camera</span>{/if}
		<span class="hint">Drag = orbit · right-drag = pan · wheel = dolly · click a figure, then drag its gizmo</span>
	</div>
	<div class="stage" bind:this={host}>
		{#if !studio.sel}
			<p class="empty">Pick a shot on the left.</p>
		{/if}
		<div class="frame" bind:this={glHost} class:hidden={!studio.sel}>
			<canvas class="texts" bind:this={overlay}></canvas>
		</div>
		{#if info}<p class="sc-error">{info}</p>{/if}
	</div>
</section>

<style>
	.view {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		min-width: 0;
	}
	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		flex-wrap: wrap;
	}
	.check {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
	}
	.hint {
		font-size: var(--type-caption-size);
		opacity: 0.7;
	}
	.state {
		font-size: var(--type-caption-size);
		padding: 0 var(--space-05);
		background: var(--ink);
		color: var(--paper);
	}
	.stage {
		flex: 1;
		min-height: 0;
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--ink);
		overflow: hidden;
	}
	/* out of the flow: the stage's size comes from the layout, the frame is fitted into it (fit()) */
	.frame {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		line-height: 0;
	}
	.frame.hidden {
		display: none;
	}
	.frame :global(canvas.gl) {
		display: block;
	}
	.texts {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}
	.empty {
		color: var(--paper);
	}
	.stage .sc-error {
		position: absolute;
		bottom: var(--space-1);
		left: var(--space-1);
		background: var(--paper);
	}
</style>
