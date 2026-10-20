// The look (#124, moved from tools/storyboard-render/core.js sinPass): a reusable pass on one renderer –
// render(scene, camera, w, h, { sky, accent, K, seed }) → the renderer's canvas. Lit + normals + a mask (R = accent,
// G = object id) → pure black / white at a hard threshold; ink only where one thing ends (object ids; depth jumps
// between set pieces), organic width, drawn inverted against what is behind it.
import * as THREE from 'three';

/** the look's parameters: the sky white (1) or black (0), the accent colour, the supersampling, the noise seed */
export interface LookParams {
	sky?: number;
	accent?: string;
	K?: number;
	seed?: number;
}
export interface SinPass {
	render(scene: THREE.Scene, cam: THREE.Camera, w: number, h: number, p?: LookParams): HTMLCanvasElement;
	dispose(): void;
}

// THE LOOK – one shader (owner 2026-10-10: "use only one shader", chose "Hatch" from solid / hatch / tone / hatch +
// inner lines; after Maxime Heckel's "Moebius-style post-processing"). Black and white, the one hard key light does
// the work: what faces it is white paper, the shadow is crosshatched – more layers the darker (diagonal <= .80,
// vertical <= .55, horizontal <= .35, solid black <= .12), stripes every `hatch` px. Lines: where one THING ends (a
// figurine against anything – object ids, no lines at joints or inside a body), depth jumps between set pieces, and
// inside the set (Sobel over depth + the normals' luma) – organic width, inverted against what is under them (white
// on black). The one accent colour where the storyboard names it.
const FRAGMENT = `
	uniform sampler2D tLit, tMask, tDepth, tNorm; uniform vec2 texel, res; uniform float sky, near, far, ortho, ink, inkVar, seed, hatch, K; uniform vec3 accent;
	varying vec2 vUv;
	float lin(float d){ if (ortho > .5) return near + d * (far - near); float z = d * 2. - 1.; return 2. * near * far / (far + near - z * (far - near)); }
	float dep(vec2 uv){ return lin(texture2D(tDepth, uv).r); }
	float oid(vec2 uv){ return floor(texture2D(tMask, uv).g * 255. + .5); }
	float luma(vec3 c){ return dot(c, vec3(.2125, .7154, .0721)); }
	float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1, 311.7)) + seed) * 43758.5453); }
	float vnoise(vec2 p){ vec2 i = floor(p), f = fract(p); f = f * f * (3. - 2. * f);
		return mix(mix(hash(i), hash(i + vec2(1., 0.)), f.x), mix(hash(i + vec2(0., 1.)), hash(i + vec2(1., 1.)), f.x), f.y); }
	float sobel(vec2 uv, vec2 st){
		float gx = 0., gy = 0., nx = 0., ny = 0.;
		float dc = max(dep(uv), .001);
		for (int j = 0; j < 3; j++) for (int i = 0; i < 3; i++) {
			float fx = float(i) - 1., fy = float(j) - 1.;
			float wx = fx * (fy == 0. ? 2. : 1.), wy = fy * (fx == 0. ? 2. : 1.);
			vec2 o = vec2(fx, fy) * st;
			float d = dep(uv + o) / dc;
			float n = luma(texture2D(tNorm, uv + o).rgb);
			gx += wx * d; gy += wy * d; nx += wx * n; ny += wy * n;
		}
		return sqrt(gx * gx + gy * gy) * 3. + sqrt(nx * nx + ny * ny);
	}
	void main(){
		vec2 px = vUv * res;
		bool empty = texture2D(tDepth, vUv).r > 0.99999;
		float l = luma(texture2D(tLit, vUv).rgb);
		// paper + crosshatching in the shadow
		float s = empty ? sky : 1.;
		if (!empty) {
			float sp = hatch * K, w = max(1., K);
			if (l <= .80 && mod(px.x + px.y, sp) <= w) s = 0.;
			if (l <= .55 && mod(px.x, sp) <= w) s = 0.;
			if (l <= .35 && mod(px.y, sp) <= w) s = 0.;
			if (l <= .12) s = 0.;
		}
		// lines: the end of a thing (ids), depth jumps between set pieces – organic width
		float n = vnoise(px / 46.) * .65 + vnoise(px / 13.) * .35;
		float width = ink + inkVar * n * n;
		float dc = dep(vUv), ic = oid(vUv), e = 0.;
		for (int r = 1; r <= 6; r++) {
			float rr = float(r);
			if (rr > width) break;
			for (int k = 0; k < 8; k++) {
				float a = float(k) * .7853982;
				vec2 q = vUv + vec2(cos(a), sin(a)) * texel * rr;
				if (oid(q) != ic) { e = 1.; break; }
				float dn = dep(q);
				if (ic < .5 && abs(dn - dc) > .06 * min(dc, dn)) { e = 1.; break; }
			}
			if (e > .5) break;
		}
		// lines inside the set (never inside a figurine)
		if (!empty && e < .5 && ic < .5 && sobel(vUv, texel * max(1., K) * 1.5) > .9) e = 1.;
		float c = e > .5 ? 1. - step(.5, s) : s;
		vec3 col = vec3(c);
		if (texture2D(tMask, vUv).r > .5 && e < .5) col = s > .5 ? accent : accent * .55;
		gl_FragColor = vec4(col, 1.);
	}`;

interface Targets {
	w: number;
	h: number;
	lit: THREE.WebGLRenderTarget;
	mask: THREE.WebGLRenderTarget;
	norm: THREE.WebGLRenderTarget;
}

export function sinPass(renderer: THREE.WebGLRenderer): SinPass {
	renderer.shadowMap.enabled = true;
	renderer.shadowMap.type = THREE.PCFShadowMap; // PCF honours shadow.radius
	renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
	const maskMats: Record<string, THREE.MeshBasicMaterial> = {};
	const normalMat = new THREE.MeshNormalMaterial();
	const PASS = new THREE.ShaderMaterial({
		uniforms: {
			tLit: { value: null }, tMask: { value: null }, tDepth: { value: null }, tNorm: { value: null },
			texel: { value: new THREE.Vector2() }, res: { value: new THREE.Vector2() }, sky: { value: 1 },
			ink: { value: 1.6 }, inkVar: { value: 2.2 }, seed: { value: 0 }, hatch: { value: 8 }, K: { value: 1 },
			accent: { value: new THREE.Color('#c00000') }, near: { value: 0.05 }, far: { value: 600 }, ortho: { value: 0 }
		},
		vertexShader: 'varying vec2 vUv; void main(){ vUv = uv; gl_Position = vec4(position.xy, 0., 1.); }',
		fragmentShader: FRAGMENT,
		depthTest: false,
		depthWrite: false
	});
	const quad = new THREE.PlaneGeometry(2, 2);
	const post = new THREE.Scene();
	post.add(new THREE.Mesh(quad, PASS));
	const postCam = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
	let rt: Targets | null = null;
	const free = (t: Targets | null) => {
		for (const x of [t?.lit, t?.mask, t?.norm]) {
			x?.depthTexture?.dispose();
			x?.dispose();
		}
	};
	const targets = (w: number, h: number): Targets => {
		if (rt && rt.w === w && rt.h === h) return rt;
		free(rt);
		const make = () => {
			const t = new THREE.WebGLRenderTarget(w, h, { depthBuffer: true });
			t.depthTexture = new THREE.DepthTexture(w, h);
			return t;
		};
		return (rt = { w, h, lit: make(), mask: make(), norm: make() });
	};
	return {
		render(scene, cam, w, h, p = {}) {
			renderer.setSize(w, h, false);
			const { lit, mask, norm } = targets(w, h);
			renderer.setClearColor(0x000000, 1);
			renderer.setRenderTarget(lit);
			renderer.render(scene, cam);
			scene.overrideMaterial = normalMat;
			renderer.setRenderTarget(norm);
			renderer.render(scene, cam);
			scene.overrideMaterial = null;
			const swap: [THREE.Mesh, THREE.Material | THREE.Material[]][] = [];
			scene.traverse((o) => {
				const m = o as THREE.Mesh;
				if (!m.isMesh || o.userData.helper) return;
				swap.push([m, m.material]);
				const k = `${o.userData.accent ? 1 : 0}/${o.userData.fig ?? 0}`;
				m.material = maskMats[k] ??= new THREE.MeshBasicMaterial({ color: new THREE.Color(o.userData.accent ? 1 : 0, (o.userData.fig ?? 0) / 255, 0) });
			});
			renderer.setRenderTarget(mask);
			renderer.render(scene, cam);
			for (const [o, m] of swap) o.material = m;
			const u = PASS.uniforms;
			u.tLit.value = lit.texture;
			u.tMask.value = mask.texture;
			u.tDepth.value = lit.depthTexture;
			u.tNorm.value = norm.texture;
			u.texel.value.set(1 / w, 1 / h);
			u.res.value.set(w, h);
			const c = cam as THREE.PerspectiveCamera | THREE.OrthographicCamera;
			u.near.value = c.near;
			u.far.value = c.far;
			u.ortho.value = (c as THREE.OrthographicCamera).isOrthographicCamera ? 1 : 0;
			u.sky.value = p.sky ?? 1;
			u.ink.value = 1.6 * (p.K ?? 1);
			u.inkVar.value = 2.2 * (p.K ?? 1);
			u.seed.value = p.seed ?? 0;
			u.K.value = p.K ?? 1;
			u.accent.value.set(p.accent ?? '#c00000');
			renderer.setRenderTarget(null);
			renderer.render(post, postCam);
			return renderer.domElement;
		},
		dispose() {
			free(rt);
			rt = null;
			PASS.dispose();
			quad.dispose();
			normalMat.dispose();
			for (const m of Object.values(maskMats)) m.dispose();
		}
	};
}
