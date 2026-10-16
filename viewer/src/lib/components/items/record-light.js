// The record stage's light (owner 2026-10-16: "a bit of light: a spot on the scene and a little ambient occlusion – as
// simple as possible but moody"): a warm spot from above-front makes a pool of light on a dark floor whose rim fades
// into the card's black; little ambient / environment (enough for the platter's metal and the vinyl's sheen); one soft
// shadow map from the spot and soft contact shadows (radial blobs) under the plinth and each sleeve – no
// post-processing. Plain JS (record-stage.js).

/** a round gradient on a small canvas: [offset, css colour] stops from the middle out */
function radial(THREE, stops) {
	const c = document.createElement('canvas');
	c.width = c.height = 128;
	const g = c.getContext('2d');
	const grad = g.createRadialGradient(64, 64, 0, 64, 64, 64);
	for (const [at, color] of stops) grad.addColorStop(at, color);
	g.fillStyle = grad;
	g.fillRect(0, 0, 128, 128);
	const tex = new THREE.CanvasTexture(c);
	tex.colorSpace = THREE.SRGBColorSpace;
	return tex;
}

/** a soft contact shadow w × d on the floor (y = 0) */
export function contactShadow(THREE, w, d, opacity = 0.8) {
	const tex = radial(THREE, [[0, `rgba(0,0,0,${opacity})`], [0.55, `rgba(0,0,0,${opacity * 0.6})`], [1, 'rgba(0,0,0,0)']]);
	const m = new THREE.Mesh(new THREE.PlaneGeometry(w, d), new THREE.MeshBasicMaterial({ map: tex, transparent: true, depthWrite: false }));
	m.rotation.x = -Math.PI / 2;
	m.position.y = 0.0006;
	m.renderOrder = 1;
	return m;
}

/** the spot, the floor, the plinth's contact shadow; `deck` = the plinth's footprint { x, z, w, d } */
export function moodyLights(THREE, renderer, scene, deck) {
	renderer.shadowMap.enabled = true;
	renderer.shadowMap.type = THREE.PCFSoftShadowMap;
	scene.environmentIntensity = 0.16;
	scene.add(new THREE.HemisphereLight(0xffffff, 0x222222, 0.12));
	const spot = new THREE.SpotLight(0xffe9d2, 13, 0, 0.42, 1, 2);
	spot.position.set(-0.04, 1.6, 0.7);
	spot.target.position.set(-0.04, 0.02, -0.06);
	spot.castShadow = true;
	spot.shadow.mapSize.set(1024, 1024);
	spot.shadow.camera.near = 0.5;
	spot.shadow.camera.far = 3;
	spot.shadow.bias = -0.0004;
	spot.shadow.normalBias = 0.01;
	scene.add(spot, spot.target);
	// the floor catches the pool of light and fades to the card's black at its rim (no horizon line)
	const pool = radial(THREE, [[0, '#ffffff'], [0.35, '#bdbdbd'], [0.75, '#2a2a2a'], [1, '#000000']]);
	const floor = new THREE.Mesh(new THREE.PlaneGeometry(2.6, 2.6), new THREE.MeshStandardMaterial({ color: 0x4a423b, map: pool, roughness: 0.95, envMapIntensity: 0 }));
	floor.rotation.x = -Math.PI / 2;
	floor.position.x = -0.06;
	floor.receiveShadow = true;
	const under = contactShadow(THREE, deck.w + 0.12, deck.d + 0.12, 0.85);
	under.position.x = deck.x;
	under.position.z = deck.z;
	scene.add(floor, under);
}

/** these cast (and take) the spot's shadow */
export function castShadows(...objs) {
	for (const o of objs) o.traverse((m) => m.isMesh && (m.castShadow = m.receiveShadow = true));
}
