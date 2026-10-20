<script lang="ts">
	// The 3D view's camera controls (#124; the studio's Viewport): drag = orbit, right-drag = pan, wheel = dolly (three's
	// OrbitControls, made new for each scene). The orbit's centre: the hand camera's target, else straight ahead at the
	// subject's depth – computed before the controls exist (they turn the camera towards it), so the view opens exactly
	// as the frame; a dutch angle survives. Never under the ground. Each orbit that really moved the camera is written
	// as the shot's hand camera { pos, target, fov } with the figures' screen places – one undo step.
	import { untrack } from 'svelte';
	import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
	import type { PerspectiveCamera } from 'three';
	import type { Shot } from '@scutoons/content/storyboard';
	import { cameraKey, MIN_CAMERA_Y, orbitCentre } from '$lib/editor/storyboard-3d.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import type { Stage3d } from '$lib/storyboard/stage.svelte.ts';

	let { stage, sb, commit }: { stage: Stage3d; sb: StoryboardPage; commit: (change: (shot: Shot) => void) => void } = $props();

	$effect(() => {
		const built = stage.built;
		if (!built) return;
		const controls = untrack(() => {
			const cam = built.camera;
			const target = sb.sel?.shot.camera?.target;
			const centre = orbitCentre(cam.position, cam.quaternion, built.light.target ?? null, target);
			const roll = cam.rotation.z;
			const c = new OrbitControls(cam, stage.renderer.domElement);
			c.enableDamping = false;
			c.target.copy(centre);
			c.update();
			if (!target && Math.abs(roll) > 1e-3 && (cam as PerspectiveCamera).isPerspectiveCamera) cam.rotateZ(roll);
			let last = cameraKey(cam.position.toArray(), c.target.toArray());
			c.addEventListener('change', () => {
				if (cam.position.y < MIN_CAMERA_Y) {
					cam.position.y = MIN_CAMERA_Y;
					cam.lookAt(c.target);
				}
				stage.draw();
			});
			c.addEventListener('end', () => {
				const edit = stage.cameraEdit(c.target, last);
				if (!edit) return;
				last = cameraKey(cam.position.toArray(), c.target.toArray());
				commit(edit.change);
			});
			stage.controls = c;
			stage.draw();
			return c;
		});
		return () => {
			controls.dispose();
			if (stage.controls === controls) stage.controls = null;
		};
	});
</script>
