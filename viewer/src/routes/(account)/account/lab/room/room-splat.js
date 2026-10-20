// A Gaussian splat room for the lab (agents/research/LOCATION-SCENES.md §3, issue #74): a `.splat` file (32 bytes a
// splat – tools/rooms/pipeline/splat.mjs) drawn with three.js, no library: the splats' centre, 3D covariance and colour
// live in data textures, one instanced quad per splat fetches them by index (WebGL2), a worker sorts the indices back to
// front when the view turns, normal alpha blending. The things in the room (pickups, residents) draw first and the
// splats test against their depth.

const VERT = /* glsl */ `
precision highp float;
precision highp int;
uniform mat4 modelViewMatrix;
uniform mat4 projectionMatrix;
uniform highp sampler2D centers;
uniform highp sampler2D covA;
uniform highp sampler2D covB;
uniform highp sampler2D colors;
uniform vec2 focal;
uniform vec2 viewport;
uniform int width;
in vec3 position;
in float splatIndex;
out vec4 vColor;
out vec2 vPos;
void main() {
	int i = int(splatIndex);
	ivec2 uv = ivec2(i % width, i / width);
	vec3 c = texelFetch(centers, uv, 0).xyz;
	vec4 cam = modelViewMatrix * vec4(c, 1.0);
	vec4 clip = projectionMatrix * cam;
	float bound = 1.2 * clip.w;
	if (cam.z > -0.05 || clip.x < -bound || clip.x > bound || clip.y < -bound || clip.y > bound) {
		gl_Position = vec4(0.0, 0.0, 2.0, 1.0);
		return;
	}
	vec3 a = texelFetch(covA, uv, 0).xyz;
	vec3 b = texelFetch(covB, uv, 0).xyz;
	mat3 V = mat3(a.x, a.y, a.z, a.y, b.x, b.y, a.z, b.y, b.z);
	mat3 J = mat3(
		focal.x / cam.z, 0.0, -(focal.x * cam.x) / (cam.z * cam.z),
		0.0, focal.y / cam.z, -(focal.y * cam.y) / (cam.z * cam.z),
		0.0, 0.0, 0.0);
	mat3 W = transpose(mat3(modelViewMatrix));
	mat3 T = W * J;
	mat3 cov = transpose(T) * V * T;
	float xx = cov[0][0] + 0.3, yy = cov[1][1] + 0.3, xy = cov[0][1];
	float mid = 0.5 * (xx + yy);
	float rad = length(vec2(0.5 * (xx - yy), xy));
	float l1 = mid + rad, l2 = max(mid - rad, 0.1);
	vec2 dir = normalize(vec2(xy, l1 - xx));
	vec2 major = min(sqrt(2.0 * l1), 1024.0) * dir;
	// +90°: (major, minor) keeps the quad counter-clockwise (a −90° basis flips it and three culls it as a back face)
	vec2 minor = min(sqrt(2.0 * l2), 1024.0) * vec2(-dir.y, dir.x);
	vColor = texelFetch(colors, uv, 0);
	vPos = position.xy;
	vec2 ndc = clip.xy / clip.w;
	gl_Position = vec4(ndc + (position.x * major + position.y * minor) * 2.0 / viewport, clip.z / clip.w, 1.0);
}`;

const FRAG = /* glsl */ `
precision highp float;
in vec4 vColor;
in vec2 vPos;
out vec4 fragColor;
void main() {
	float A = -dot(vPos, vPos);
	if (A < -4.0) discard;
	float alpha = exp(A) * vColor.a;
	if (alpha < 1.0 / 255.0) discard;
	fragColor = vec4(vColor.rgb, alpha);
}`;

// the depth sort, in a worker: counting sort on 16 bits, far → near
const SORTER = `
let centers = null, count = 0;
onmessage = (e) => {
	if (e.data.centers) { centers = e.data.centers; count = e.data.count; return; }
	const v = e.data.view;
	const depth = new Int32Array(count);
	let min = Infinity, max = -Infinity;
	for (let i = 0; i < count; i++) {
		const d = v[2] * centers[i * 3] + v[6] * centers[i * 3 + 1] + v[10] * centers[i * 3 + 2];
		depth[i] = d * 4096;
		if (depth[i] < min) min = depth[i];
		if (depth[i] > max) max = depth[i];
	}
	const k = 65535 / Math.max(1, max - min), counts = new Uint32Array(65536);
	for (let i = 0; i < count; i++) counts[(depth[i] = ((depth[i] - min) * k) | 0)]++;
	const starts = new Uint32Array(65536);
	for (let i = 1; i < 65536; i++) starts[i] = starts[i - 1] + counts[i - 1];
	const order = new Float32Array(count);
	for (let i = 0; i < count; i++) order[starts[depth[i]]++] = i;
	postMessage({ order }, [order.buffer]);
};`;

/** loads a `.splat` and returns { mesh, update(camera, renderer), dispose, count } */
export async function loadSplat(THREE, url) {
	const res = await fetch(url);
	if (!res.ok) throw new Error(`${res.status} ${url}`);
	const bytes = await res.arrayBuffer();
	const count = bytes.byteLength / 32;
	const f = new Float32Array(bytes),
		u = new Uint8Array(bytes);
	const width = 2048,
		height = Math.ceil(count / width);
	const centers = new Float32Array(width * height * 4),
		covA = new Float32Array(width * height * 4),
		covB = new Float32Array(width * height * 4);
	const colors = new Uint8Array(width * height * 4);
	const plain = new Float32Array(count * 3);
	const q = new THREE.Quaternion(),
		m = new THREE.Matrix3(),
		R = new THREE.Matrix4();
	for (let i = 0; i < count; i++) {
		const o = i * 8;
		centers.set([f[o], f[o + 1], f[o + 2], 1], i * 4);
		plain.set([f[o], f[o + 1], f[o + 2]], i * 3);
		colors.set(u.subarray(i * 32 + 24, i * 32 + 28), i * 4);
		const b = i * 32 + 28;
		q.set(
			(u[b + 1] - 128) / 128,
			(u[b + 2] - 128) / 128,
			(u[b + 3] - 128) / 128,
			(u[b] - 128) / 128
		).normalize();
		R.makeRotationFromQuaternion(q);
		m.setFromMatrix4(R);
		const e = m.elements,
			s = [f[o + 3], f[o + 4], f[o + 5]];
		// M = R · S, Σ = M Mᵀ (column-major elements)
		const M = [
			e[0] * s[0],
			e[1] * s[0],
			e[2] * s[0],
			e[3] * s[1],
			e[4] * s[1],
			e[5] * s[1],
			e[6] * s[2],
			e[7] * s[2],
			e[8] * s[2]
		];
		const dot = (r, c) => M[r] * M[c] + M[r + 3] * M[c + 3] + M[r + 6] * M[c + 6];
		covA.set([dot(0, 0), dot(0, 1), dot(0, 2), 0], i * 4);
		covB.set([dot(1, 1), dot(1, 2), dot(2, 2), 0], i * 4);
	}
	const tex = (data, type, format) => {
		const t = new THREE.DataTexture(data, width, height, format, type);
		t.needsUpdate = true;
		return t;
	};
	const uniforms = {
		centers: { value: tex(centers, THREE.FloatType, THREE.RGBAFormat) },
		covA: { value: tex(covA, THREE.FloatType, THREE.RGBAFormat) },
		covB: { value: tex(covB, THREE.FloatType, THREE.RGBAFormat) },
		colors: { value: tex(colors, THREE.UnsignedByteType, THREE.RGBAFormat) },
		focal: { value: new THREE.Vector2() },
		viewport: { value: new THREE.Vector2() },
		width: { value: width }
	};
	uniforms.colors.value.colorSpace = THREE.NoColorSpace;
	const geometry = new THREE.InstancedBufferGeometry();
	geometry.setAttribute(
		'position',
		new THREE.BufferAttribute(new Float32Array([-2, -2, 0, 2, -2, 0, 2, 2, 0, -2, 2, 0]), 3)
	);
	geometry.setIndex([0, 1, 2, 0, 2, 3]);
	const index = new THREE.InstancedBufferAttribute(
		new Float32Array(count).map((_, i) => i),
		1
	);
	index.setUsage(THREE.DynamicDrawUsage);
	geometry.setAttribute('splatIndex', index);
	geometry.instanceCount = count;
	const material = new THREE.RawShaderMaterial({
		uniforms,
		vertexShader: VERT,
		fragmentShader: FRAG,
		glslVersion: THREE.GLSL3,
		transparent: true,
		depthWrite: false,
		depthTest: true,
		blending: THREE.NormalBlending
	});
	const mesh = new THREE.Mesh(geometry, material);
	mesh.frustumCulled = false;
	mesh.renderOrder = 10;

	const worker = new Worker(URL.createObjectURL(new Blob([SORTER], { type: 'text/javascript' })));
	worker.postMessage({ centers: plain, count });
	let busy = false,
		last = null;
	worker.onmessage = (e) => {
		index.array.set(e.data.order);
		index.needsUpdate = true;
		busy = false;
	};
	const size = new THREE.Vector2();
	return {
		mesh,
		count,
		mb: bytes.byteLength / 1e6,
		/** per frame: the projection's focal length in pixels; a resort when the view turned */
		update(camera, renderer) {
			renderer.getDrawingBufferSize(size);
			uniforms.viewport.value.copy(size);
			uniforms.focal.value.set(
				(camera.projectionMatrix.elements[0] * size.x) / 2,
				(camera.projectionMatrix.elements[5] * size.y) / 2
			);
			const v = camera.matrixWorldInverse.elements;
			const key = [v[2], v[6], v[10], v[14]].map((x) => x.toFixed(3)).join();
			if (!busy && key !== last) {
				busy = true;
				last = key;
				worker.postMessage({ view: Float32Array.from(v) });
			}
		},
		dispose() {
			worker.terminate();
			geometry.dispose();
			material.dispose();
			for (const u of Object.values(uniforms)) u.value?.dispose?.();
		}
	};
}
