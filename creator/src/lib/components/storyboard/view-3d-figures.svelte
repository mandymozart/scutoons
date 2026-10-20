<script lang="ts">
	// The 3D view's figure gestures (#124; the studio's Viewport – owner 2026-10-10: "i was able to move things before for
	// corrections, that was nicer"): a figure is handled directly – drag it = move on its ground (the Turn tool: drag
	// sideways = turn about its feet), wheel on it = its size about its feet, Alt + wheel = turn by 15°. A click on a
	// figure picks it (its gizmo), a click on the empty view drops the pick. Its place is written once per gesture (on
	// release, the wheel once it rests) as x / y / h / turn – the exact inverse of the renderer's placing. The listeners
	// sit on the canvas before the camera controls' (made later, per scene): a gesture on a figure never orbits.
	import { onMount } from 'svelte';
	import { Plane, Vector3 } from 'three';
	import type { Shot } from '@scutoons/content/storyboard';
	import { yawOf } from '$lib/editor/storyboard-3d-place.ts';
	import { CLICK_SLOP, DRAG_SLOP, dragTurn, moveOnGround, scaleAbout, travelled, turnAbout, WHEEL_REST, wheelFactor, wheelTurn } from '$lib/editor/storyboard-3d.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import type { Stage3d } from '$lib/storyboard/stage.svelte.ts';

	let {
		stage,
		sb,
		tool,
		figure = $bindable(),
		commit
	}: { stage: Stage3d; sb: StoryboardPage; tool: 'move' | 'turn'; figure: number | null; commit: (change: (shot: Shot) => void) => void } = $props();

	// the stage lives as long as the view (one per page)
	// svelte-ignore state_referenced_locally
	const canvas = stage.renderer.domElement;
	/** a figure's place written back (one undo step) */
	function write(i: number) {
		const change = stage.figureEdit(i);
		if (change && sb.sel?.shot.figures?.[i]) commit(change);
	}

	interface Grab {
		i: number;
		plane: Plane;
		start: Vector3 | null;
		from: Vector3;
		yaw: number;
		x: number;
		y: number;
		moved: boolean;
	}
	let grab: Grab | null = null;
	let down: { x: number; y: number } | null = null;

	function onDown(e: PointerEvent) {
		down = null;
		if (!stage.built || stage.held || e.button !== 0 || stage.gizmo?.axis) return; // a gizmo handle under the pointer: the gizmo's
		const i = stage.figureAt(e);
		if (i < 0) {
			down = { x: e.clientX, y: e.clientY };
			return;
		}
		e.stopImmediatePropagation();
		e.preventDefault();
		figure = i;
		const pivot = stage.pivots[i];
		const plane = new Plane(new Vector3(0, 1, 0), -stage.anchorOf(i).y);
		grab = { i, plane, start: stage.onPlane(e, plane), from: pivot.position.clone(), yaw: yawOf(pivot.quaternion), x: e.clientX, y: e.clientY, moved: false };
		canvas.style.cursor = 'grabbing';
		window.addEventListener('pointermove', onDrag);
		window.addEventListener('pointerup', onDrop, { once: true });
		stage.draw();
	}
	function onDrag(e: PointerEvent) {
		if (!grab) return;
		if (!grab.moved && travelled(grab.x, grab.y, e.clientX, e.clientY) < DRAG_SLOP) return;
		grab.moved = true;
		const pivot = stage.pivots[grab.i];
		if (!pivot) return;
		if (tool === 'turn') turnAbout(pivot, stage.anchors[grab.i], dragTurn(grab.yaw, e.clientX - grab.x));
		else {
			const p = grab.start && stage.onPlane(e, grab.plane);
			if (!p || !grab.start) return;
			pivot.position.copy(moveOnGround(grab.from, grab.start, p));
		}
		stage.draw();
	}
	function onDrop() {
		window.removeEventListener('pointermove', onDrag);
		canvas.style.cursor = '';
		const g = grab;
		grab = null;
		if (g?.moved) write(g.i);
	}
	// a click on the empty view (no drag, not on the gizmo) drops the picked figure
	function onUp(e: PointerEvent) {
		if (!down || travelled(down.x, down.y, e.clientX, e.clientY) > CLICK_SLOP || stage.gizmo?.dragging) return;
		down = null;
		figure = null;
		stage.draw();
	}

	let wheelTimer: ReturnType<typeof setTimeout> | 0 = 0;
	let wheeled = -1;
	function onWheel(e: WheelEvent) {
		if (!stage.built || stage.held) return; // a pose holds the camera (#126): the figures rest
		const i = stage.figureAt(e);
		if (i < 0) return; // the camera's dolly
		e.preventDefault();
		e.stopImmediatePropagation();
		if (wheelTimer && wheeled !== i) settleWheel();
		figure = i;
		wheeled = i;
		const pivot = stage.pivots[i];
		const m = stage.built.figs[i];
		if (e.altKey) turnAbout(pivot, stage.anchors[i], yawOf(pivot.quaternion) + wheelTurn(e.deltaY, e.deltaX));
		else {
			const k = wheelFactor(e.deltaY);
			m.root.position.copy(scaleAbout(m.root.position, stage.anchors[i], k));
			m.root.scale.multiplyScalar(k);
			if (m.place) m.place.scale *= k;
		}
		stage.draw();
		if (wheelTimer) clearTimeout(wheelTimer);
		wheelTimer = setTimeout(settleWheel, WHEEL_REST);
	}
	/** the wheel's gesture ends: written once */
	function settleWheel() {
		if (wheelTimer) clearTimeout(wheelTimer);
		wheelTimer = 0;
		if (wheeled >= 0) write(wheeled);
		wheeled = -1;
	}

	// a grab hand over a figure
	let hoverFrame = 0;
	function onHover(e: PointerEvent) {
		if (grab || hoverFrame) return;
		hoverFrame = requestAnimationFrame(() => {
			hoverFrame = 0;
			if (!grab) canvas.style.cursor = !stage.gizmo?.axis && stage.figureAt(e) >= 0 ? 'grab' : '';
		});
	}

	onMount(() => {
		canvas.addEventListener('pointerdown', onDown);
		canvas.addEventListener('pointerup', onUp);
		canvas.addEventListener('wheel', onWheel, { passive: false });
		canvas.addEventListener('pointermove', onHover);
		return () => {
			// a gesture still open when the view goes: written first
			if (grab) onDrop();
			if (wheelTimer) settleWheel();
			cancelAnimationFrame(hoverFrame);
			canvas.removeEventListener('pointerdown', onDown);
			canvas.removeEventListener('pointerup', onUp);
			canvas.removeEventListener('wheel', onWheel);
			canvas.removeEventListener('pointermove', onHover);
		};
	});
	// another scene (another shot, a field changed): an open wheel gesture belongs to the old one
	$effect(() => {
		void stage.built;
		return () => {
			if (wheelTimer) clearTimeout(wheelTimer);
			wheelTimer = 0;
			wheeled = -1;
		};
	});
</script>
