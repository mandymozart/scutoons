<script>
	// The 3D view (owner 2026-10-09: "can this be just a free moving camera in a 3d view since we already use threejs?"):
	// the shot's scene (the frames' own renderer, tools/storyboard-render/core.js) with the Sin City pass live. Drag =
	// orbit, right-drag / Shift = pan, wheel = dolly; the camera is the shot's camera – every move is written back as
	// shot.camera { pos, target, fov }. A figure is handled directly (like the first studio, owner 2026-10-10: "i was able
	// to move things before for corrections, that was nicer"): drag it = move on the ground (Turn tool: drag sideways =
	// turn), wheel on it = size, Alt + wheel = turn; the gizmo of the picked figure works too. Its place is written back
	// once per gesture as the storyboard's screen position (x, y, h, turn) – the exact inverse of buildScene's placing,
	// so a rebuild (any field changed) puts every figure exactly where it was.
	// The texts (the SCUTOONS presets) on top, switchable. The handheld camera drives the same camera (handheldView):
	// while a pose holds it the orbit, the gizmo and the figure gestures rest – nothing is measured or written through a
	// take's camera; "Shot camera" gives it back.
	// Over 400 lines: the camera, the figure gestures and their write-back share one scene and one camera.
	import * as THREE from 'three';
	import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
	import { TransformControls } from 'three/addons/controls/TransformControls.js';
	import { onDestroy, onMount } from 'svelte';
	import { Segmented } from '@scutoons/ui';
	import { buildScene, drawDialog, figureBoxes, loadFonts, seedOf, sinPass } from '../../../../../tools/storyboard-render/core.js';
	import { specOf, studio, touched } from '../studio.svelte.js';
	import { handheldView } from '../handheld/view.js';
	import { endPreview, hh } from '../handheld/handheld.svelte.js';

	const D = Math.PI / 180;
	const FACE = { front: 0, away: 180, left: -90, right: 90, 'three-quarter-left': -40, 'three-quarter-right': 40 }; // as core.js
	let host = $state();
	let glHost = $state();
	let overlay = $state();
	let tool = $state('move');
	let texts = $state(true);
	let info = $state('');
	const how = $derived(`Drag = orbit · right-drag = pan · wheel = dolly · on a figure: drag = ${tool === 'turn' ? 'turn' : 'move'} · wheel = size · Alt + wheel = turn`);
	const short = $derived(`drag orbit · right-drag pan · wheel dolly · figure: drag ${tool === 'turn' ? 'turn' : 'move'}`);
	let renderer, pass, built, controls, gizmo, helperScene, spec, look;
	let pivots = [];
	const lookOf = () => ({ sky: built?.skyWhite ? 1 : 0, accent: spec?.accent ?? '#c00000', K: 1, seed: seedOf(spec?.id ?? '') });
	let size = { w: 0, h: 0, scale: 1 };
	let lastKey = '';
	let selfKey = '';
	let building = false, again = false;

	// for checks in the browser (dev only): the scene, the camera, the orbit
	if (import.meta.env.DEV) window.__studioView = { get built() { return built; }, get controls() { return controls; }, get spec() { return spec; }, get size() { return size; }, get gizmo() { return gizmo; }, get pivots() { return pivots; }, place: (i) => screenPlace(i) };
	onMount(() => {
		renderer = new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
		renderer.domElement.classList.add('gl');
		glHost.appendChild(renderer.domElement);
		pass = sinPass(renderer);
		helperScene = new THREE.Scene();
		renderer.domElement.addEventListener('pointerdown', onPick);
		// the figure gestures go first (capture on the frame): OrbitControls and the gizmo never see them
		glHost.addEventListener('pointerdown', onGrab, { capture: true });
		glHost.addEventListener('wheel', onWheel, { capture: true, passive: false });
		renderer.domElement.addEventListener('pointermove', onHover);
		const ro = new ResizeObserver(() => { fit(); draw(); });
		ro.observe(host);
		loadFonts();
		// the handheld camera (HandheldPanel): the phone's pose / a take moves this camera; release = back where it was
		handheldView.api = {
			camera: () => built?.camera ?? null,
			pose(s) {
				if (!built || building) return;
				const cam = built.camera;
				if (!held) { settle(); hold({ p: cam.position.clone(), q: cam.quaternion.clone(), fov: cam.fov }); }
				place(cam, s);
				draw();
			},
			// the trim's in / out pair (TrimPair via handheld stillAt): the picture into a 2D canvas – at a pose `s`
			// (rendered without the gizmo, the camera back where it was after) or as the view shows it now; false while
			// the scene builds
			still(canvas, s) {
				if (!built || building || !size.w) return false;
				if (canvas.width !== size.w || canvas.height !== size.h) [canvas.width, canvas.height] = [size.w, size.h];
				const g = canvas.getContext('2d');
				g.clearRect(0, 0, size.w, size.h);
				if (!s) {
					g.drawImage(renderer.domElement, 0, 0);
					if (overlay) g.drawImage(overlay, 0, 0);
					return true;
				}
				const cam = built.camera;
				const was = { position: cam.position.toArray(), quaternion: cam.quaternion.toArray(), fov: cam.fov };
				place(cam, s);
				pass.render(built.scene, cam, size.w, size.h, look);
				g.drawImage(renderer.domElement, 0, 0);
				if (texts) drawDialog(g, spec, figureBoxes(built.figs, cam, size.w, size.h, 1), size.w, size.h, size.scale);
				place(cam, was);
				draw();
				return true;
			},
			release() {
				if (!held || !built) { hold(null); return; }
				const cam = built.camera;
				cam.position.copy(held.p);
				cam.quaternion.copy(held.q);
				if (cam.isPerspectiveCamera) { cam.fov = held.fov; cam.updateProjectionMatrix(); }
				cam.updateMatrixWorld();
				hold(null);
				draw();
			}
		};
		return () => { ro.disconnect(); handheldView.api = null; hold(null); };
	});
	let held = null; // the camera before the handheld took it over
	// a pose (a take's sample, the phone's: { position, quaternion, fov }) onto the camera
	function place(cam, s) {
		cam.position.fromArray(s.position);
		cam.quaternion.fromArray(s.quaternion);
		if (s.fov && cam.isPerspectiveCamera && Math.abs(cam.fov - s.fov) > 1e-3) { cam.fov = s.fov; cam.updateProjectionMatrix(); }
		cam.updateMatrixWorld();
	}
	// held: the orbit and the gizmo rest (the figure gestures check `held` themselves); hh.held follows (view.js onheld)
	function hold(h) {
		held = h;
		if (controls) controls.enabled = !h;
		if (gizmo) gizmo.enabled = !h;
		handheldView.onheld?.(!!h);
	}
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
		const bw = host.clientWidth - 8, bh = host.clientHeight - 8; // a hair of the stage around the frame
		const scale = Math.min(bw / spec.outW, bh / spec.outH, 2.5);
		size = { w: Math.max(32, Math.round(spec.outW * scale)), h: Math.max(32, Math.round(spec.outH * scale)), scale };
		// rem of the tools' fluid root (tools.css) – never px in CSS (owner's dogma)
		const root = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
		const rem = (px) => `${px / root}rem`;
		renderer.domElement.style.width = rem(size.w);
		renderer.domElement.style.height = rem(size.h);
		if (overlay) { overlay.width = size.w; overlay.height = size.h; overlay.style.width = rem(size.w); overlay.style.height = rem(size.h); }
	}

	let busy = $state(false);
	async function rebuild() {
		if (building) { again = true; return; }
		building = true;
		busy = true;
		try {
			spec = specOf(1);
			controls?.dispose(); gizmo?.dispose(); helperScene.clear(); built?.dispose();
			hold(null);
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
				// the yaw buildScene gave it (read BEFORE attach: attach re-derives the Euler and flips a yaw past 90° into x = z = 180°)
				m.yaw = m.root.rotation.y;
				const pivot = new THREE.Group();
				pivot.position.copy(c);
				built.scene.add(pivot);
				pivot.attach(m.root);
				m.anchor = pivot.worldToLocal(m.place.pos.clone()); // buildScene's anchor (the feet on the ground), in the pivot
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
			controls.update();
			lastCam = [...cam.position.toArray(), ...controls.target.toArray()].map((n) => n.toFixed(3)).join();
			if (!t && Math.abs(roll) > 1e-3 && cam.isPerspectiveCamera) cam.rotateZ(roll); // a dutch angle survives
			// never under the ground (a worm's-eye camera sits just above it)
			controls.addEventListener('change', () => { if (cam.position.y < 0.08) { cam.position.y = 0.08; cam.lookAt(controls.target); } draw(); });
			controls.addEventListener('end', commitCamera);
			gizmo = new TransformControls(cam, renderer.domElement);
			gizmo.size = 0.8;
			gizmo.addEventListener('dragging-changed', (e) => (controls.enabled = !e.value && !held));
			gizmo.addEventListener('objectChange', draw);
			gizmo.addEventListener('mouseUp', () => commitFigures()); // not the listener itself: the event would be its `i` (nothing written)
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
		if (held) return; // an orbit a pose cut short: the hand camera is the shot's, never a take's
		const cam = built.camera;
		const shot = studio.sel.shot;
		// a click ends an orbit too – only a camera that really moved is written
		const now = [...cam.position.toArray(), ...controls.target.toArray()].map((n) => n.toFixed(3)).join();
		if (now === lastCam) return;
		lastCam = now;
		const pos = cam.position.toArray().map((n) => +n.toFixed(3));
		const target = controls.target.toArray().map((n) => +n.toFixed(3));
		// the camera as it will be rebuilt (rounded) – the figures are measured through exactly that one
		cam.position.fromArray(pos);
		controls.target.fromArray(target);
		cam.lookAt(controls.target);
		cam.updateMatrixWorld();
		shot.camera = { pos, target, fov: +cam.fov.toFixed(1) };
		writeFigures(); // the figures stay where they are in the world: their screen places follow the camera
		selfKey = JSON.stringify($state.snapshot(shot));
		touched();
		draw();
	}
	// a figure's world place → the storyboard's screen place: the exact inverse of buildScene (core.js) – its anchor
	// (feet on the ground, else the centre of what the frame shows) seen through the camera, the height in screens from
	// its scale and depth, the turn against the camera. A rebuild from these numbers puts it back where it is.
	function screenPlace(i) {
		const m = built.figs[i];
		const cam = built.camera;
		cam.updateMatrixWorld();
		const W = spec.w, H = spec.h, K = built.K ?? 1;
		const pos = pivots[i].localToWorld(m.anchor.clone());
		const ndc = pos.clone().project(cam);
		const px = ((ndc.x + 1) / 2) * W, py = ((1 - ndc.y) / 2) * H;
		const { size, scale, grounded } = m.place;
		let hPx;
		if (cam.isPerspectiveCamera) {
			const fpx = H / 2 / Math.tan((cam.fov * D) / 2);
			const depth = grounded ? -pos.clone().applyMatrix4(cam.matrixWorldInverse).z : pos.distanceTo(cam.position);
			hPx = (scale * size * fpx) / depth;
		} else hPx = scale * (H / (cam.top - cam.bottom));
		const cy = grounded ? py - hPx / 2 : py;
		const yaw = m.yaw + yawOf(pivots[i]);
		const toCam = Math.atan2(cam.position.x - pos.x, cam.position.z - pos.z);
		const turn = ((((yaw - toCam) / D) % 360) + 540) % 360 - 180;
		// a figure floating (placed along its ray) that this camera would put on the ground: it says so (core.js ground)
		let float = false;
		if (!grounded && !m.place.free && cam.isPerspectiveCamera) {
			const foot = new THREE.Vector3((px / W) * 2 - 1, -((cy + hPx / 2) / H) * 2 + 1, 0.5).unproject(cam).sub(cam.position).normalize();
			float = foot.y < -0.01;
		}
		return { x: px / W, y: cy / H, h: hPx / (844 * K), turn, float };
	}
	const UP = new THREE.Vector3(0, 1, 0);
	const yawOf = (o) => new THREE.Euler().setFromQuaternion(o.quaternion, 'YXZ').y;
	// turn a figure to a yaw about its own feet (its anchor stays put, so a turn never shifts it)
	function turnTo(i, yaw) {
		const pv = pivots[i];
		const anchor = pv.localToWorld(built.figs[i].anchor.clone());
		pv.quaternion.setFromAxisAngle(UP, yaw);
		pv.updateMatrixWorld();
		pv.position.add(anchor.sub(pv.localToWorld(built.figs[i].anchor.clone())));
	}
	const r4 = (n) => +n.toFixed(4);
	function writeFigures(only = null) {
		const shot = studio.sel.shot;
		built.scene.updateMatrixWorld(true);
		built.figs.forEach((m, i) => {
			if (only != null && i !== only) return; // a figure moved by hand: only it is written
			const f = shot.figures?.[i];
			if (!f || !m.place) return;
			const p = screenPlace(i);
			f.x = r4(p.x);
			f.y = r4(p.y);
			f.h = r4(p.h);
			if (p.float) f.ground = false;
			else delete f.ground;
			// the turn only when facing (or the turn it has) doesn't already give it
			const was = f.turn ?? FACE[f.facing] ?? 0;
			const off = Math.abs(((((p.turn - was) % 360) + 540) % 360) - 180);
			if (off > 0.05) f.turn = +p.turn.toFixed(1);
		});
	}
	function commitFigures(i = studio.figure) {
		if (i == null || !built || held) return; // measured through the shot's camera only (screenPlace)
		writeFigures(i);
		selfKey = JSON.stringify($state.snapshot(studio.sel.shot));
		touched();
		draw();
	}

	const ndcOf = (e) => {
		const r = renderer.domElement.getBoundingClientRect();
		return new THREE.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
	};
	// the figure under the pointer (index) or -1
	function figureAt(e) {
		if (!built) return -1;
		// a skinned figure keeps the bounds it was first measured with (posed elsewhere, before it was placed / moved): a
		// ray then misses it everywhere – measured fresh for each pick
		for (const m of built.figs) m.root.traverse((o) => { if (o.isSkinnedMesh) { o.boundingBox = null; o.boundingSphere = null; } });
		const ray = new THREE.Raycaster();
		ray.setFromCamera(ndcOf(e), built.camera);
		const hit = ray.intersectObjects(built.figs.map((m) => m.root), true).find((h) => h.object.userData.fig);
		return hit ? hit.object.userData.fig - 1 : -1;
	}
	function pickFigure(i) {
		studio.figure = i;
		if (gizmo && pivots[i]) gizmo.attach(pivots[i]);
	}
	// where the pointer meets a horizontal plane (the figure's ground)
	function onPlane(e, plane) {
		const ray = new THREE.Raycaster();
		ray.setFromCamera(ndcOf(e), built.camera);
		const p = ray.ray.intersectPlane(plane, new THREE.Vector3());
		return p && p.distanceTo(built.camera.position) < 300 ? p : null;
	}

	// drag a figure itself: Move = along its ground under the pointer, Turn = sideways turns it; written once on release
	let grab = null;
	function onGrab(e) {
		if (!built || building || held || e.button !== 0 || gizmo?.axis) return; // a gizmo handle under the pointer: the gizmo's
		const i = figureAt(e);
		if (i < 0) return;
		e.stopPropagation();
		e.preventDefault();
		pickFigure(i);
		const pv = pivots[i];
		const anchor = pv.localToWorld(built.figs[i].anchor.clone());
		const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -anchor.y);
		grab = { i, plane, start: onPlane(e, plane), from: pv.position.clone(), yaw: yawOf(pv), x: e.clientX, y: e.clientY, moved: false };
		host.style.cursor = 'grabbing';
		window.addEventListener('pointermove', onDrag);
		window.addEventListener('pointerup', onDrop, { once: true });
		draw();
	}
	function onDrag(e) {
		if (!grab) return;
		if (!grab.moved && Math.hypot(e.clientX - grab.x, e.clientY - grab.y) < 3) return;
		grab.moved = true;
		const pv = pivots[grab.i];
		if (tool === 'turn') turnTo(grab.i, grab.yaw + (e.clientX - grab.x) * 0.01);
		else {
			const p = grab.start && onPlane(e, grab.plane);
			if (!p) return;
			pv.position.set(grab.from.x + p.x - grab.start.x, grab.from.y, grab.from.z + p.z - grab.start.z);
		}
		draw();
	}
	function onDrop() {
		window.removeEventListener('pointermove', onDrag);
		host.style.cursor = '';
		const g = grab;
		grab = null;
		if (g?.moved) commitFigures(g.i);
	}
	// wheel on a figure = its size (around its feet), Alt + wheel = turn by 15°; written once the wheel rests
	let wheelTimer;
	function onWheel(e) {
		if (!built || building || held) return;
		const i = figureAt(e);
		if (i < 0) return;
		e.preventDefault();
		e.stopPropagation();
		if (studio.figure !== i) pickFigure(i);
		const m = built.figs[i];
		if (e.altKey) turnTo(i, yawOf(pivots[i]) + Math.sign(e.deltaY || e.deltaX) * 15 * D);
		else {
			const k = e.deltaY < 0 ? 1.05 : 1 / 1.05;
			m.root.position.sub(m.anchor).multiplyScalar(k).add(m.anchor);
			m.root.scale.multiplyScalar(k);
			m.place.scale *= k;
		}
		draw();
		clearTimeout(wheelTimer);
		wheelTimer = setTimeout(() => { wheelTimer = 0; commitFigures(i); }, 250);
	}
	// a grab hand over a figure
	let hoverFrame = 0;
	function onHover(e) {
		if (grab || hoverFrame) return;
		hoverFrame = requestAnimationFrame(() => {
			hoverFrame = 0;
			if (!grab && host) host.style.cursor = !held && !gizmo?.axis && figureAt(e) >= 0 ? 'grab' : '';
		});
	}

	// a gesture still open when a pose takes the camera ends first – written through the shot's camera. The gizmo's too:
	// disabled mid-drag it would miss its pointerup and drag on with the bare pointer once the camera is back
	function settle() {
		if (gizmo?.dragging) gizmo.pointerUp(null); // → mouseUp → commitFigures
		if (grab) onDrop();
		if (wheelTimer) { clearTimeout(wheelTimer); wheelTimer = 0; commitFigures(); }
		commitCamera(); // an orbit under way (only a camera that moved is written)
	}

	// a click on the empty view (no drag, not on the gizmo) drops the picked figure – figures themselves are onGrab's
	let down = null;
	function onPick(e) {
		down = gizmo?.axis ? null : { x: e.clientX, y: e.clientY };
		renderer.domElement.addEventListener('pointerup', (u) => {
			if (!down || Math.hypot(u.clientX - down.x, u.clientY - down.y) > 4 || gizmo?.dragging) return;
			studio.figure = null;
			gizmo?.detach();
			draw();
		}, { once: true });
	}

	// back to the camera the shot's size / angle give: a take's moment holding the view lets go first (Play stops); only a
	// hand camera is removed – a shot without one stays as it is (not unsaved for nothing)
	function resetCamera() {
		endPreview();
		if (!studio.sel?.shot.camera) return;
		delete studio.sel.shot.camera;
		touched();
	}
</script>

<section class="view" aria-label="3D view">
	<div class="tools">
		<Segmented label="Figure tool" options={[{ value: 'move', label: 'Move', icon: 'move' }, { value: 'turn', label: 'Turn', icon: 'rotate-cw' }]} value={tool} onchange={(v) => (tool = v)} />
		<Segmented label="Texts" options={[{ value: 'on', label: 'Texts', icon: 'type', title: 'The texts (SCUTOONS presets) on top' }]} value={texts ? 'on' : null} onchange={() => (texts = true)} onclear={() => (texts = false)} />
		<button type="button" class="secondary" onclick={resetCamera} disabled={!studio.sel?.shot.camera && !hh.held} title="Back to the camera the shot's size / angle give (removes the hand camera, ends a take's preview)">Shot camera</button>
		{#if busy}<span class="state" role="status">rendering …</span>{/if}
		{#if studio.sel?.shot.camera}<span class="state" title="The shot has its own camera from the 3D view – 'Shot camera' goes back to size + angle">hand camera</span>{/if}
		<span class="hint" title={how}><span>{short}</span></span>
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
		gap: var(--space-05);
		padding: var(--space-025) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		flex-wrap: wrap;
	}
	/* one line in the room the tools leave, the full text in its tooltip – the view keeps its height. Where that room is
	   narrower than the line (≈ 29.7rem at the caption size) it goes as a whole instead of being cut mid-word
	   (STUDIO-LAYOUT §1 problem 6); the tooltip stays on the empty room. The ellipsis only for a wider font elsewhere. */
	.hint {
		flex: 1 1 0;
		min-width: 0;
		container-type: inline-size;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-size: var(--type-caption-size);
		opacity: 0.7;
	}
	@container (max-width: 30rem) {
		.hint > span {
			display: none;
		}
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
