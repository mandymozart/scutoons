import * as THREE from "three";
import {
  AdditiveBlending,
  Box3,
  BufferAttribute,
  BufferGeometry,
  Color,
  type Light,
  type Material,
  Matrix4,
  type Mesh,
  type Object3D,
  Points,
  ShaderMaterial,
  type SkinnedMesh,
  Vector3,
  type WebGLProgramParametersWithUniforms,
} from "three";
import { MeshSurfaceSampler } from "three/examples/jsm/math/MeshSurfaceSampler.js";
import {
  type Appearance,
  type AppearanceData,
  APPEARANCES,
  DEFAULT_APPEARANCE_TYPE,
  isBuiltInAppearanceType,
  type Moment,
  resolveAppearance,
  type ShapeEasing,
} from "../types/index";
import { type AppearanceRegistration, appearanceRegistration } from "./appearances";

/**
 * Appear / disappear animations of an AR entity, at three moments: `reveal` (the first find of a target), `found`
 * (every later find) and `lost` (the page is lost – the entity goes, then the anchor hides). What plays is the **look**
 * the content names for the moment (`experience.appearance`, `shared/types/appearance.ts`): a plugin's own
 * (`./appearances`), or a built-in one, played here from its values in three parts, each with its own share of the time –
 *
 *   - **shape**: the entity starts flat on the page and / or small and turned, and rises to its height and size. A
 *     transform of the entity's object in the anchor's space (the page: z is its normal), around the centre of the
 *     entity's footprint – so it holds for every entity type, a skinned model included.
 *   - **mask**: the Codrops emissive dissolve (Jatin Chopra, "Implementing a Dissolve Effect with Shaders and
 *     Particles in Three.js", 2025 – github.com/JatinChopra/emissive-dissolve-effect, MIT): a front moves through 3D
 *     noise; what it has not reached is discarded, the edge at the front takes the edge colour, the band behind it
 *     shines and cools down, sparks fly from the surface where the front passes.
 *   - **surface**: a tint that fades away, an opacity fade.
 *
 * `reveal` and `found` play a look forwards, `lost` plays it rewound in the look's own time for it.
 * Works for the entity's whole node tree and every node type (model, video / image plane, chroma key): the materials
 * are copied for the animation (model materials are shared with the cached asset) with the injection
 * (onBeforeCompile); the originals come back afterwards.
 */

/** The moment an animation plays at */
export type AnimationKind = Moment;

const isColor = (value: unknown): value is string => typeof value === "string" && /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i.test(value);

/**
 * The values of a built-in look: the type's defaults with the content's values over them, each taken only where it is
 * of its parameter's kind (the reader's content is checked before; a tool hands loose values). null for a type that is
 * not built in.
 */
export const lookOf = (data: AppearanceData): Appearance | null => {
  if (!isBuiltInAppearanceType(data.type)) return null;
  const spec = APPEARANCES[data.type];
  const look: Record<string, unknown> = {};
  for (const [name, param] of Object.entries(spec.params)) {
    const value = data[name];
    const ok = param.kind === "number" ? typeof value === "number" && Number.isFinite(value)
      : param.kind === "color" ? isColor(value)
      : param.kind === "choice" ? param.options.includes(value as string)
      : false;
    look[name] = ok ? value : param.default;
  }
  return look as unknown as Appearance;
};

/** The look a moment plays where the content names none */
const DEFAULT_LOOK: AppearanceData = { type: DEFAULT_APPEARANCE_TYPE };

/** A plugin's look that names no usable duration takes this long (ms) */
const PLUGIN_LOOK_MS = 600;

/** The noise spans −amplitude … amplitude: twice a pattern's 0 … 1 */
const NOISE_AMPLITUDE = 1;
const PATTERN = 2;
/** The flat entity keeps this share of its height (faces in one plane would flicker), the smallest this share of its size */
const MIN_HEIGHT = 0.004;
const MIN_SCALE = 0.001;

const clamp01 = (x: number) => Math.min(1, Math.max(0, x));
/** Where `u` is between `from` and `to`, 0 → 1 */
const span = (u: number, from: number, to: number) => (to > from ? clamp01((u - from) / (to - from)) : u >= to ? 1 : 0);
const smoothstep = (x: number) => {
  const t = clamp01(x);
  return t * t * (3 - 2 * t);
};
const EASINGS: Record<ShapeEasing, (t: number) => number> = {
  linear: t => t,
  smooth: smoothstep,
  out: t => 1 - (1 - t) ** 3,
  inOut: t => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2),
  in: t => t * t * t,
};

/** 3D simplex noise (Ashima Arts / Stefan Gustavson, MIT) → about −1…1 */
const NOISE_GLSL = /* glsl */ `
vec3 dsMod289(vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 dsMod289(vec4 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
vec4 dsPermute(vec4 x) { return dsMod289(((x * 34.0) + 10.0) * x); }
vec4 dsTaylorInvSqrt(vec4 r) { return 1.79284291400159 - 0.85373472095314 * r; }
float dsNoise(vec3 v) {
  const vec2 C = vec2(1.0 / 6.0, 1.0 / 3.0);
  const vec4 D = vec4(0.0, 0.5, 1.0, 2.0);
  vec3 i = floor(v + dot(v, C.yyy));
  vec3 x0 = v - i + dot(i, C.xxx);
  vec3 g = step(x0.yzx, x0.xyz);
  vec3 l = 1.0 - g;
  vec3 i1 = min(g.xyz, l.zxy);
  vec3 i2 = max(g.xyz, l.zxy);
  vec3 x1 = x0 - i1 + C.xxx;
  vec3 x2 = x0 - i2 + C.yyy;
  vec3 x3 = x0 - D.yyy;
  i = dsMod289(i);
  vec4 p = dsPermute(dsPermute(dsPermute(i.z + vec4(0.0, i1.z, i2.z, 1.0)) + i.y + vec4(0.0, i1.y, i2.y, 1.0)) + i.x + vec4(0.0, i1.x, i2.x, 1.0));
  float n_ = 0.142857142857;
  vec3 ns = n_ * D.wyz - D.xzx;
  vec4 j = p - 49.0 * floor(p * ns.z * ns.z);
  vec4 x_ = floor(j * ns.z);
  vec4 y_ = floor(j - 7.0 * x_);
  vec4 x = x_ * ns.x + ns.yyyy;
  vec4 y = y_ * ns.x + ns.yyyy;
  vec4 h = 1.0 - abs(x) - abs(y);
  vec4 b0 = vec4(x.xy, y.xy);
  vec4 b1 = vec4(x.zw, y.zw);
  vec4 s0 = floor(b0) * 2.0 + 1.0;
  vec4 s1 = floor(b1) * 2.0 + 1.0;
  vec4 sh = -step(h, vec4(0.0));
  vec4 a0 = b0.xzyw + s0.xzyw * sh.xxyy;
  vec4 a1 = b1.xzyw + s1.xzyw * sh.zzww;
  vec3 p0 = vec3(a0.xy, h.x);
  vec3 p1 = vec3(a0.zw, h.y);
  vec3 p2 = vec3(a1.xy, h.z);
  vec3 p3 = vec3(a1.zw, h.w);
  vec4 norm = dsTaylorInvSqrt(vec4(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
  p0 *= norm.x; p1 *= norm.y; p2 *= norm.z; p3 *= norm.w;
  vec4 m = max(0.5 - vec4(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), 0.0);
  m = m * m;
  return 42.0 * dot(m * m, vec4(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
}
`;

/** Uniforms of the entity's materials during an animation */
export interface DissolveUniforms {
  /** 1 = the noise dissolve runs, 0 = every fragment is drawn */
  uDissolve: { value: number };
  uFreq: { value: number };
  uAmp: { value: number };
  uProgress: { value: number };
  uEdge: { value: number };
  uEdgeColor: { value: Color };
  uGlowWidth: { value: number };
  uGlowColor: { value: Color };
  /** 0–1: strength of the glow behind the edge */
  uGlow: { value: number };
  /** 0–1: how much of the tint colour lies over the surface */
  uTint: { value: number };
  uTintColor: { value: Color };
  uOpacity: { value: number };
}

/**
 * Add the animation to a material's shaders: a position in the entity's space (`uToEntity` – the same for
 * every mesh of the entity, so the noise is continuous over the whole object); with the dissolve, fragments below
 * the progress are discarded, the edge band takes the edge colour, the band behind it shines in the glow colour and
 * cools down; then the tint and the opacity.
 */
export const injectDissolve = (
  shader: Pick<WebGLProgramParametersWithUniforms, "vertexShader" | "fragmentShader" | "uniforms">,
  uniforms: DissolveUniforms,
  toEntity: Matrix4,
): void => {
  Object.assign(shader.uniforms, uniforms, { uToEntity: { value: toEntity } });
  shader.vertexShader = shader.vertexShader
    .replace("void main() {", "uniform mat4 uToEntity;\nvarying vec3 vDissolvePosition;\nvoid main() {\n  vDissolvePosition = (uToEntity * vec4(position, 1.0)).xyz;");
  const declarations = [
    "uniform float uDissolve;", "uniform float uFreq;", "uniform float uAmp;", "uniform float uProgress;", "uniform float uEdge;",
    "uniform vec3 uEdgeColor;", "uniform float uGlowWidth;", "uniform vec3 uGlowColor;", "uniform float uGlow;",
    "uniform float uTint;", "uniform vec3 uTintColor;", "uniform float uOpacity;",
    "varying vec3 vDissolvePosition;",
  ].join("\n");
  shader.fragmentShader = `${declarations}\n${NOISE_GLSL}\n${shader.fragmentShader}`.replace(/\}\s*$/, `
  if (uDissolve > 0.5) {
    float dissolveNoise = dsNoise(vDissolvePosition * uFreq) * uAmp;
    if (dissolveNoise < uProgress) discard;
    // Freshly materialised areas shine and cool down behind the edge
    float dissolveGlow = 1.0 - smoothstep(uProgress + uEdge, uProgress + uEdge + uGlowWidth, dissolveNoise);
    gl_FragColor.rgb = mix(gl_FragColor.rgb, uGlowColor, uGlow * dissolveGlow * dissolveGlow);
    // The edge
    if (dissolveNoise < uProgress + uEdge) gl_FragColor = vec4(uEdgeColor, 1.0);
  }
  gl_FragColor.rgb = mix(gl_FragColor.rgb, uTintColor, uTint);
  gl_FragColor.a *= uOpacity;
}`);
};

const sparkVertexShader = /* glsl */ `
uniform float uFreq;
uniform float uAmp;
uniform float uByNoise;
uniform float uSparkProgress;
uniform float uSize;
uniform float uDistance;
uniform float uViewportHalfHeight;
uniform mat4 uRest;
uniform mat4 uShape;
attribute vec3 aDirection;
attribute float aRandom;
varying float vLife;
${NOISE_GLSL}
void main() {
  // Born when the dissolve's front passes its point on the surface – without a dissolve, scene over the shape's
  // time –, flies off, fades
  float born = uByNoise > 0.5 ? 0.5 - 0.5 * dsNoise(position * uFreq) * uAmp : fract(aRandom * 7.13);
  float life = (uSparkProgress - born) / (0.25 + 0.2 * aRandom);
  vLife = life;
  // It starts on the surface as the entity is shaped right now and flies unshaped (the anchor's space)
  vec3 origin = (uShape * uRest * vec4(position, 1.0)).xyz;
  vec3 direction = normalize(mat3(uRest) * aDirection);
  vec3 p = origin + direction * (1.0 - pow(1.0 - clamp(life, 0.0, 1.0), 3.0)) * uDistance * (0.4 + aRandom);
  vec4 mvPosition = modelViewMatrix * vec4(p, 1.0);
  gl_Position = projectionMatrix * mvPosition;
  float visible = step(0.0, life) * step(life, 1.0);
  // Perspective size: uSize is in the anchor's units (the model-view matrix scales them)
  float viewScale = length(modelViewMatrix[0].xyz);
  gl_PointSize = visible * uSize * viewScale * projectionMatrix[1][1] * uViewportHalfHeight / -mvPosition.z
    * (0.5 + aRandom) * (1.0 - life);
}
`;

const sparkFragmentShader = /* glsl */ `
uniform vec3 uColor;
varying float vLife;
void main() {
  // Round, soft dot (no pow() of a negative number – undefined on some mobile GPUs)
  float d = length(gl_PointCoord - vec2(0.5));
  float edge = clamp(1.0 - d * 2.0, 0.0, 1.0);
  if (edge <= 0.0) discard;
  float glow = edge * edge;
  // Brighter than white while young: the sparks bloom too
  gl_FragColor = vec4(uColor * (2.5 - 2.0 * vLife), glow * (1.0 - vLife));
}
`;

/** Sparks on the entity's surface: positions and flight directions (normals) in the entity's space */
const sampleSparks = (meshes: Array<{ mesh: Mesh; toEntity: Matrix4 }>, count: number): BufferGeometry | null => {
  const positions: number[] = [];
  const directions: number[] = [];
  const randoms: number[] = [];
  const position = new Vector3();
  const normal = new Vector3();
  const perMesh = Math.max(1, Math.floor(count / Math.max(1, meshes.length)));
  let seed = 1;
  const random = () => {
    const x = Math.sin(seed++ * 12.9898) * 43758.5453; // deterministic
    return x - Math.floor(x);
  };
  for (const { mesh, toEntity } of meshes) {
    const positionAttribute = mesh.geometry.getAttribute("position");
    if (!positionAttribute || positionAttribute.count < 3) continue;
    const hasNormals = !!mesh.geometry.getAttribute("normal");
    let sampler: MeshSurfaceSampler;
    try {
      sampler = new MeshSurfaceSampler(mesh);
      // Deterministic sparks (the method exists, @types/three lacks it)
      (sampler as unknown as { setRandomGenerator(fn: () => number): void }).setRandomGenerator(random);
      sampler.build();
    } catch {
      continue; // e.g. no triangles
    }
    const normalMatrix = new Matrix4().copy(toEntity).invert().transpose();
    for (let i = 0; i < perMesh; i++) {
      if (hasNormals) sampler.sample(position, normal);
      else {
        sampler.sample(position);
        normal.set(0, 0, 1);
      }
      position.applyMatrix4(toEntity);
      normal.transformDirection(normalMatrix);
      // Mostly along the surface normal, a little random
      normal.add(new Vector3(random() - 0.5, random() - 0.5, random() - 0.5).multiplyScalar(0.8)).normalize();
      positions.push(position.x, position.y, position.z);
      directions.push(normal.x, normal.y, normal.z);
      randoms.push(random());
    }
  }
  if (!positions.length) return null;
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new BufferAttribute(new Float32Array(positions), 3));
  geometry.setAttribute("aDirection", new BufferAttribute(new Float32Array(directions), 3));
  geometry.setAttribute("aRandom", new BufferAttribute(new Float32Array(randoms), 1));
  return geometry;
};

export interface Celebration {
  readonly kind: AnimationKind;
  /** Advance by `delta` seconds – false once it is done and cleaned up */
  update(delta: number): boolean;
  /** Bloom strength wanted right now (the view adds the bloom while any animation wants it) */
  readonly bloom: number;
  /** End now: originals back, sparks removed (e.g. the scene changes, the target is found again) */
  finish(): void;
}

/**
 * The meshes' box in another space (`inverse`: the inverse of that space's world matrix), from their vertices as they
 * are posed now – a box of the meshes' world boxes would be too large for a turned model, and would change with the
 * camera's angle. A sample of the vertices of a large mesh.
 */
const boundsIn = (meshes: Mesh[], inverse: Matrix4): Box3 => {
  const box = new Box3();
  const point = new Vector3();
  const toSpace = new Matrix4();
  for (const mesh of meshes) {
    const position = mesh.geometry.getAttribute("position");
    if (!position) continue;
    (mesh as SkinnedMesh).skeleton?.update(); // the bones' matrices, before the first frame drew them
    toSpace.multiplyMatrices(inverse, mesh.matrixWorld);
    const step = Math.max(1, Math.floor(position.count / 4000));
    for (let i = 0; i < position.count; i += step) box.expandByPoint(mesh.getVertexPosition(i, point).applyMatrix4(toSpace));
  }
  return box;
};

/**
 * Visit `root`'s node tree – except the subtrees whose top is in `exclude` (the nodes with a look of their own: they are
 * animated apart, with theirs)
 */
const walk = (root: Object3D, visit: (object: Object3D) => void, exclude?: ReadonlySet<Object3D>): void => {
  visit(root);
  root.children.forEach(child => {
    if (!exclude?.has(child)) walk(child, visit, exclude);
  });
};

/**
 * How much of an entity its look shows right now, for the code on it: `userData.shown` (0…1) on the object a look
 * animates – 0 nothing yet, 1 the entity as it is – written every frame of a reveal, a find and a loss, by the
 * built-in looks and a plugin's alike, and taken away at the end (no value = all of it). A node's share is the product
 * of the values on it and its ancestors (a node with a look of its own animates apart, inside its parent's). A
 * component that should come and go with its entity – a contact shadow – reads it (docs/plugins.md "A look").
 */
export const SHOWN = "shown";

/**
 * What a look tells the entity every frame: its lights' share of their intensity (`userData.fade`, applied by the
 * light node – entities.ts) and `userData.shown` on the entity; `done` puts both back
 */
const tell = (entity: Object3D, exclude?: ReadonlySet<Object3D>) => {
  const lights: Light[] = [];
  walk(entity, object => {
    if ((object as Light).isLight) lights.push(object as Light);
  }, exclude);
  return {
    shown: (visible: number) => {
      lights.forEach(light => (light.userData.fade = visible));
      entity.userData[SHOWN] = visible;
    },
    done: () => {
      lights.forEach(light => (light.userData.fade = 1));
      delete entity.userData[SHOWN];
    },
  };
};

/** Every mesh of the node tree (but the excluded subtrees) */
const meshesOf = (entity: Object3D, exclude?: ReadonlySet<Object3D>): Mesh[] => {
  const meshes: Mesh[] = [];
  walk(entity, object => {
    if ((object as Mesh).isMesh) meshes.push(object as Mesh);
  }, exclude);
  return meshes;
};

/**
 * Copies of the meshes' materials for one animation, on the meshes until `restore` (model materials are shared with
 * the cached asset – an animation never changes those). `prepare` gets every copy before it is first drawn.
 */
const swapMaterials = (meshes: Mesh[], prepare: (clone: Material, original: Material, mesh: Mesh) => void = () => {}) => {
  const swapped = meshes.map(mesh => {
    const original = mesh.material;
    const clones = (Array.isArray(original) ? original : [original]).map(material => {
      const clone = material.clone();
      if (material instanceof ShaderMaterial && clone instanceof ShaderMaterial) {
        clone.uniforms = { ...material.uniforms }; // keep the same textures (clone() would copy them)
      }
      prepare(clone, material, mesh);
      return clone;
    });
    mesh.material = Array.isArray(original) ? clones : clones[0];
    return { mesh, original, clones };
  });
  return {
    clones: swapped.flatMap(({ clones }) => clones),
    restore: () => swapped.forEach(({ mesh, original, clones }) => {
      mesh.material = original;
      clones.forEach(clone => clone.dispose());
    }),
  };
};

/** Sparks born last fly on for this much of the pattern after the front has passed */
const SPARK_TAIL = 0.45;

/** A built-in look's animation: shape, mask and surface from its values (the module's head) */
const playLook = (anchor: Object3D, entity: Object3D, moment: Moment, look: Appearance, exclude?: ReadonlySet<Object3D>): Celebration => {
  const reverse = moment === "lost";
  const ms = Math.max(1, reverse ? look.outDuration : look.duration);
  const masked = look.mask === "noise";
  anchor.updateWorldMatrix(true, false);
  entity.updateWorldMatrix(true, true);
  const entityInverse = new Matrix4().copy(entity.matrixWorld).invert();
  const anchorInverse = new Matrix4().copy(anchor.matrixWorld).invert();
  const drawn = meshesOf(entity, exclude);

  // Size of the entity in its own space → frequency: the same number of noise waves on every object
  const box = boundsIn(drawn, entityInverse);
  const size = box.isEmpty() ? 1 : Math.max(1e-3, box.getSize(new Vector3()).length());

  const uniforms: DissolveUniforms = {
    uDissolve: { value: masked ? 1 : 0 },
    uFreq: { value: look.waves / size },
    uAmp: { value: NOISE_AMPLITUDE },
    uProgress: { value: 1 },
    uEdge: { value: 0 },
    uEdgeColor: { value: new Color(look.edgeColor).multiplyScalar(look.edgeIntensity) },
    uGlowWidth: { value: Math.max(look.glowWidth * PATTERN, 1e-3) },
    uGlowColor: { value: new Color(look.glowColor) },
    uGlow: { value: look.glow },
    uTint: { value: 0 },
    uTintColor: { value: new Color(look.tintColor) },
    uOpacity: { value: 1 },
  };

  // Materials: a copy per mesh with the injection; the originals come back at the end. The position every mesh hands
  // the noise is in the entity's space, so the noise is continuous over the whole node tree
  const meshes = drawn.map(mesh => ({ mesh, toEntity: new Matrix4().multiplyMatrices(entityInverse, mesh.matrixWorld) }));
  const blended = new Map<Material, boolean>();
  const swap = swapMaterials(drawn, (clone, original, mesh) => {
    const toEntity = meshes.find(entry => entry.mesh === mesh)!.toEntity;
    // A material with its own injection (a plugin's layers) keeps it under the dissolve
    const previous = original.onBeforeCompile;
    clone.onBeforeCompile = (shader, renderer) => {
      previous.call(original, shader, renderer);
      injectDissolve(shader, uniforms, toEntity);
    };
    clone.customProgramCacheKey = () => `dissolve-${original.customProgramCacheKey()}`;
    blended.set(clone, original.transparent);
  });

  // Lights (light nodes) and the code on the entity listen to how much of it is there; all of it again at the end
  const told = tell(entity, exclude);

  // Shape: the entity's object is transformed in the anchor's space around the centre of its footprint (the middle
  // of its box on the page, at its base). The matrix is the shape × the placement, so the placement stays its own
  const bounds = boundsIn(drawn, anchorInverse);
  const shaped = !bounds.isEmpty() && (look.heightFrom !== 1 || look.scaleFrom !== 1 || look.spin !== 0);
  const pivot = shaped ? new Vector3((bounds.min.x + bounds.max.x) / 2, (bounds.min.y + bounds.max.y) / 2, bounds.min.z) : new Vector3();
  const shape = new Matrix4();
  const placement = new Matrix4();
  const toPivot = new Matrix4().makeTranslation(-pivot.x, -pivot.y, -pivot.z);
  const fromPivot = new Matrix4().makeTranslation(pivot.x, pivot.y, pivot.z);
  const turn = new Matrix4();
  const stretch = new Matrix4();
  const toParent = new Matrix4();
  const autoUpdate = entity.matrixAutoUpdate;
  if (shaped) entity.matrixAutoUpdate = false;
  /** The shape at its progress `k` (0 = the start: flat, small, turned; 1 = the entity as it is) */
  const shapeAt = (k: number) => {
    if (!shaped) return;
    const scale = Math.max(MIN_SCALE, look.scaleFrom + (1 - look.scaleFrom) * k);
    const height = Math.max(MIN_HEIGHT, look.heightFrom + (1 - look.heightFrom) * k);
    stretch.makeScale(scale, scale, scale * height);
    turn.makeRotationZ(look.spin * 2 * Math.PI * (1 - k));
    shape.copy(fromPivot).multiply(turn).multiply(stretch).multiply(toPivot);
    // The placement as it is now: a tool may move the entity while it appears. The shape lives in the page's space, the matrix
    // in the parent's: for a node below the root the shape is carried over (parent⁻¹ · shape · parent · placement); the
    // root's parent is the page itself
    placement.compose(entity.position, entity.quaternion, entity.scale);
    if (entity.parent && entity.parent !== anchor) {
      anchor.updateWorldMatrix(true, false);
      entity.parent.updateWorldMatrix(true, false);
      toParent.copy(anchor.matrixWorld).invert().multiply(entity.parent.matrixWorld);
      entity.matrix.copy(toParent).invert().multiply(shape).multiply(toParent).multiply(placement);
    } else {
      entity.matrix.multiplyMatrices(shape, placement);
    }
    entity.matrixWorldNeedsUpdate = true;
  };

  // Sparks in the anchor's space – children of the anchor: they follow tracking, start on the shaped surface and fly
  // unshaped. Sampled on the entity as it is placed now
  const sparkGeometry = look.sparks > 0 ? sampleSparks(meshes, look.sparks) : null;
  const sparkProgress = { value: 0 };
  const sparkMaterial = sparkGeometry && new ShaderMaterial({
    uniforms: {
      uFreq: uniforms.uFreq,
      uAmp: uniforms.uAmp,
      uByNoise: uniforms.uDissolve,
      uSparkProgress: sparkProgress,
      uSize: { value: look.sparkSize },
      uDistance: { value: look.sparkDistance },
      uColor: { value: new Color(look.sparkColor) },
      uViewportHalfHeight: { value: (window.innerHeight * Math.min(window.devicePixelRatio, 2)) / 2 },
      uRest: { value: new Matrix4().multiplyMatrices(anchorInverse, entity.matrixWorld) },
      uShape: { value: shape },
    },
    vertexShader: sparkVertexShader,
    fragmentShader: sparkFragmentShader,
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
  });
  const sparks = sparkGeometry && sparkMaterial ? new Points(sparkGeometry, sparkMaterial) : null;
  if (sparks) {
    sparks.frustumCulled = false;
    anchor.add(sparks);
  }

  // The front runs from before the pattern's start to past its end, edge and glow included
  const reach = 1.001 + Math.max(look.edgeStart, look.edgeEnd) + (look.glow > 0 ? look.glowWidth : 0);
  let elapsed = 0;
  let bloom = 0;
  let done = false;

  /** Everything at `u`: 0 = nothing there yet, 1 = the entity as it is */
  const show = (u: number) => {
    const m = smoothstep(span(u, look.maskStart, look.maskEnd));
    const front = -0.001 + (reach + 0.001) * m;
    uniforms.uProgress.value = (1 - PATTERN * front) * NOISE_AMPLITUDE;
    uniforms.uEdge.value = PATTERN * (look.edgeStart + (look.edgeEnd - look.edgeStart) * m);

    const shapeShare = span(u, look.shapeStart, look.shapeEnd);
    const k = EASINGS[look.shapeEase](shapeShare);
    shapeAt(k);

    const opacity = look.fade > 0 ? smoothstep(u / look.fade) : 1;
    uniforms.uOpacity.value = opacity;
    uniforms.uTint.value = look.tint * (1 - smoothstep(span(u, 0, look.tintEnd)));
    // Blended only while it is see-through; a material that blends anyway keeps doing so
    blended.forEach((transparent, clone) => (clone.transparent = transparent || opacity < 1));

    // Sparks: with the front, and on for a while after it has passed; without a dissolve over the shape's time
    sparkProgress.value = (masked ? front : shapeShare) + SPARK_TAIL * span(u, masked ? look.maskEnd : look.shapeEnd, 1);
    if (sparks) sparks.visible = u > 0 && u < 1;

    // Bloom: rises fast, holds, fades towards the end
    bloom = u <= 0 || u >= 1 ? 0 : look.bloom * smoothstep(u / 0.15) * (1 - smoothstep((u - 0.7) / 0.3));
    // How much is there: what makes the entity visible – the dissolve, else the fade, else the shape
    told.shown(masked ? m : look.fade > 0 ? opacity : shaped ? clamp01(k) : 1);
  };
  show(reverse ? 1 : 0);

  const finish = () => {
    if (done) return;
    done = true;
    swap.restore();
    told.done();
    if (shaped) {
      // The placement alone again
      entity.matrixAutoUpdate = autoUpdate;
      entity.updateMatrix();
      entity.matrixWorldNeedsUpdate = true;
    }
    sparks?.removeFromParent();
    sparkGeometry?.dispose();
    sparkMaterial?.dispose();
    bloom = 0;
  };

  return {
    kind: moment,
    get bloom() {
      return bloom;
    },
    finish,
    update(delta: number): boolean {
      // At most one 30 fps frame per step: the first animation compiles the injected shaders (a stall of a few
      // hundred ms on phones) – the animation waits instead of jumping past its start
      elapsed += Math.min(delta, 1 / 30) * 1000;
      const t = Math.min(1, elapsed / ms);
      show(reverse ? 1 - t : t);
      if (t < 1) return true;
      finish();
      return false;
    },
  };
};

/**
 * A plugin's look: its factory's run driven over its duration – `visible` from 0 to 1, for `lost` from 1 to 0. What it
 * set on the entity's placement and the material copies it asked for are put back at the end; a look that throws ends
 * there, with the entity as it is.
 */
const playPlugin = (
  registration: AppearanceRegistration, anchor: Object3D, entity: Object3D, moment: Moment, data: AppearanceData, exclude?: ReadonlySet<Object3D>,
): Celebration => {
  const drawn = meshesOf(entity, exclude);
  let swap: ReturnType<typeof swapMaterials> | null = null;
  const placed = { position: entity.position.clone(), quaternion: entity.quaternion.clone(), scale: entity.scale.clone(), visible: entity.visible };
  // The look's `visible` reaches the lights and the code on the entity as a built-in look's does
  const told = tell(entity, exclude);
  const run = registration.factory({
    object: entity,
    anchor,
    params: resolveAppearance(registration.spec, data),
    moment,
    three: THREE,
    materials: () => (swap ??= swapMaterials(drawn)).clones,
  });
  let done = false;
  const finish = () => {
    if (done) return;
    done = true;
    try {
      run?.dispose?.();
    } catch (error) {
      console.warn(`[celebration] The look "${data.type}" failed in dispose:`, error);
    }
    swap?.restore();
    told.done();
    entity.position.copy(placed.position);
    entity.quaternion.copy(placed.quaternion);
    entity.scale.copy(placed.scale);
    entity.visible = placed.visible;
  };
  if (!run || typeof run.update !== "function") {
    finish();
    throw new Error(`the look "${data.type}" returned no { duration, update }`);
  }
  const duration = Number(run.duration);
  const ms = Number.isFinite(duration) && duration > 0 ? duration : PLUGIN_LOOK_MS;
  const show = (t: number) => {
    const visible = moment === "lost" ? 1 - t : t;
    told.shown(visible);
    run.update(visible);
  };
  try {
    show(0);
  } catch (error) {
    finish();
    throw error;
  }
  let elapsed = 0;
  return {
    kind: moment,
    bloom: 0,
    finish,
    update(delta: number): boolean {
      if (done) return false;
      elapsed += Math.min(delta, 1 / 30) * 1000;
      const t = Math.min(1, elapsed / ms);
      try {
        show(t);
      } catch (error) {
        console.warn(`[celebration] The look "${data.type}" failed – ended:`, error);
        finish();
        return false;
      }
      if (t < 1) return true;
      finish();
      return false;
    },
  };
};

/**
 * Start the animation of a moment on `entity` – the placement group of a node of the entity's tree (the root's or a child
 * node's: its subtree animates, except the subtrees in `exclude` – nodes that animate with a look of their own), below `anchor`.
 * `look` is what the content names for the moment (`momentAppearance`): a built-in look with its values, or a plugin's;
 * a type nobody knows, or a plugin's look that fails to start, plays the default look. `lost` ends with the entity
 * gone – hide it before its materials come back.
 */
export const celebrate = (
  anchor: Object3D, entity: Object3D, moment: Moment = "reveal", look: AppearanceData = DEFAULT_LOOK, exclude?: ReadonlySet<Object3D>,
): Celebration => {
  const registration = appearanceRegistration(look.type);
  if (registration) {
    try {
      return playPlugin(registration, anchor, entity, moment, look, exclude);
    } catch (error) {
      console.warn(`[celebration] The look "${look.type}" failed to start – the default look plays:`, error);
    }
  } else if (!isBuiltInAppearanceType(look.type)) {
    console.warn(`[celebration] No look "${look.type}" – the default look plays`);
  }
  return playLook(anchor, entity, moment, lookOf(look) ?? lookOf(DEFAULT_LOOK)!, exclude);
};

/** Several animations as one: the nodes of an entity that animate each with their own look */
export const together = (moment: Moment, parts: readonly Celebration[]): Celebration => ({
  kind: moment,
  get bloom() {
    return Math.max(0, ...parts.map(part => part.bloom));
  },
  finish: () => parts.forEach(part => part.finish()),
  update(delta: number): boolean {
    // Every part advances, whether or not another is still running
    return parts.map(part => part.update(delta)).some(Boolean);
  },
});
