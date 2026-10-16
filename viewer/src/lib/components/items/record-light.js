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

/**
 * The spot, the table, the plinth's contact shadow; `deck` = the plinth's footprint { x, z, w, d }. The table runs out of
 * the picture at the bottom (owner 2026-10-16: the card's text lies on it – "the player is the brown of the
 * background"): a fill light keeps its front lit; only its far rim fades into the black. `table` = a picture for its
 * surface (a map texture – the owner compares plain and textured), else plain brown.
 */
export function moodyLights(THREE, renderer, scene, deck, table = null) {
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
	// the fill: a wide, soft warm light on the table's front, where the card's text lies
	const fill = new THREE.SpotLight(0xffe2c4, 5, 0, 0.75, 1, 2);
	fill.position.set(0, 1.4, 1.6);
	fill.target.position.set(0, 0, 0.75);
	scene.add(spot, spot.target, fill, fill.target);
	const material = new THREE.MeshStandardMaterial({ color: 0x4a423b, roughness: 0.95, envMapIntensity: 0 });
	if (table) {
		new THREE.TextureLoader().load(table, (tex) => {
			tex.colorSpace = THREE.SRGBColorSpace;
			tex.wrapS = tex.wrapT = THREE.MirroredRepeatWrapping;
			tex.repeat.set(3, 3);
			tex.anisotropy = 4;
			material.map = tex;
			material.color.set(0x8a8580);
			material.needsUpdate = true;
		});
	}
	const floor = new THREE.Mesh(new THREE.PlaneGeometry(4, 4), material);
	floor.rotation.x = -Math.PI / 2;
	floor.position.set(-0.05, 0, 0.5);
	floor.receiveShadow = true;
	// the far rim fades into the card's black (no horizon line): a black ring over the table's edge
	const rim = new THREE.Mesh(
		new THREE.PlaneGeometry(4, 4),
		new THREE.MeshBasicMaterial({ map: radial(THREE, [[0, 'rgba(0,0,0,0)'], [0.45, 'rgba(0,0,0,0)'], [0.85, 'rgba(0,0,0,0.92)'], [1, 'rgba(0,0,0,1)']]), transparent: true, depthWrite: false })
	);
	rim.rotation.x = -Math.PI / 2;
	rim.position.set(-0.05, 0.0003, 0.5);
	const under = contactShadow(THREE, deck.w + 0.12, deck.d + 0.12, 0.85);
	under.position.x = deck.x;
	under.position.z = deck.z;
	scene.add(floor, rim, under);
}

/** these cast (and take) the spot's shadow */
export function castShadows(...objs) {
	for (const o of objs) o.traverse((m) => m.isMesh && (m.castShadow = m.receiveShadow = true));
}
