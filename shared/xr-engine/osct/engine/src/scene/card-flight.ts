import {
  BufferAttribute,
  CanvasTexture,
  CatmullRomCurve3,
  FrontSide,
  Group,
  type Matrix4,
  Mesh,
  MeshBasicMaterial,
  type PerspectiveCamera,
  PlaneGeometry,
  Quaternion,
  SRGBColorSpace,
  type Texture,
  Vector3,
} from "three";

/**
 * The marginalia card (#285): the printed picture peels off the page as a playing card, rises, spins about its
 * vertical axis (the back shows, then the front again), and slaps against the screen dead centre – where it turns into
 * the entry's own picture in full resolution (`reveal`), with a rumble.
 *
 * All in the camera's space (the three.js camera stays at the origin, a target's anchor is its pose relative to the
 * camera, 1 unit = the image's width – `tracker.ts`), so only the start moves while the card flies:
 *   start  P0 = the anchor's position, n = its +z (the page's normal), q0 = its rotation, w = its scale (image width)
 *   end    P1 = the camera's (0, 0, −d), q1 = the camera's rotation (the card faces the reader, upright)
 *   path   a centripetal Catmull-Rom through P0 → a slide up the page → up off it (+ lift·n) → a turn on the way
 *          to a point straight behind the landing → P1: the rise (slide and lift, easing out to a hover), the spin
 *          (easing in and out), the slap (a steep ease-in – fastest at the screen). The page's points follow the
 *          anchor each frame; their weight is gone by the slap, so the tracking's jitter never reaches the landing.
 *   turn   q(t) = slerp(q0, q1, w(t)) · peel · spinY · roll
 *   bend   paper until the spin is over: the top edge curls up off the page (the bottom edge stays down), then it
 *          turns into a convex bow that breathes and twists a little; it stiffens into a card early in the slap
 *   time   0.8 s from a close page to 1.2 s from a far one (`flightMs`); all of it fluid, only the smash is hard
 *   impact after landing (`IMPACT_MS`): the front becomes the entry's picture, the card is pushed at the reader and
 *          bounces back, shakes, the phone vibrates once.
 */

/** Share of the screen's width and height the card may fill at the end */
export const CARD_FILL = 0.62;
/** The flight from the page to the screen: short from a close page, longer from a far one (`flightMs`) */
export const CARD_FLIGHT_MS = { near: 800, far: 1200 } as const;
/** The slap: push, bounce and shake after the card reached the screen */
export const IMPACT_MS = 480;
/** How long the card stays in the centre after the impact (the entry opens over it) */
export const CARD_HOLD_MS = 250;
/** Where the rise ends and the slap starts (0…1 of the flight) */
const RISE_END = 0.28;
const SLAP_START = 0.8;
/** Up off the page, in card widths; how far the turn swings out to the side */
const LIFT = 0.6;
/** First the card slides up the page (towards its top edge), barely off it, in card widths */
const SLIDE = 0.45;
const SWING = 0.18;
/** The slap starts from this far behind the landing, in end distances */
const BEHIND = 1.9;
/** The bend's strongest curl of the top edge, radians */
const CURL = 1.8;
/** The twist's strongest turn between the bottom and the top edge, radians */
const TWIST = 0.55;
/** Bend segments along the card's height */
const BEND_SEGMENTS = 18;

/** The anchor's pose, split up */
export interface CardStart {
  position: Vector3;
  rotation: Quaternion;
  /** The card's width in the scene (the anchor's scale) */
  width: number;
  /** The page's normal (the anchor's +z) */
  normal: Vector3;
}

export const startFromAnchor = (matrix: Matrix4): CardStart => {
  const position = new Vector3();
  const rotation = new Quaternion();
  const scale = new Vector3();
  matrix.decompose(position, rotation, scale);
  return { position, rotation, width: scale.x, normal: new Vector3(0, 0, 1).applyQuaternion(rotation) };
};

/**
 * How far in front of the camera a card `width` wide and `width · aspect` high fills `fill` of the screen in both
 * directions: d = max(w / (2f·tan(hFov/2)), h / (2f·tan(vFov/2))). The tangents come from the projection matrix (the
 * tracker sets the matrix, not `fov`); never closer than just beyond the near plane.
 */
export const endDistance = (projection: Matrix4, width: number, aspect: number, fill = CARD_FILL): number => {
  const e = projection.elements;
  const tanHalfH = 1 / e[0];
  const tanHalfV = 1 / e[5];
  const near = e[14] / (e[10] - 1);
  const d = Math.max(width / (2 * fill * tanHalfH), (width * aspect) / (2 * fill * tanHalfV));
  return Math.max(d, Number.isFinite(near) && near > 0 ? near * 1.5 : 0);
};

const clamp01 = (x: number): number => Math.min(1, Math.max(0, x));
export const easeInOutCubic = (t: number): number => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);
const easeOutCubic = (t: number): number => 1 - (1 - t) ** 3;
const easeInOutSine = (t: number): number => -(Math.cos(Math.PI * t) - 1) / 2;

/** The three phases' local progress (each 0…1) at `t` of the flight */
export const phases = (t: number): { rise: number; spin: number; slap: number } => ({
  rise: clamp01(t / RISE_END),
  spin: clamp01((t - RISE_END) / (SLAP_START - RISE_END)),
  slap: clamp01((t - SLAP_START) / (1 - SLAP_START)),
});

/** Before the slap the card walks the first three legs of four; at the slap's start its speed carries straight on */
const BEFORE_SLAP = 0.75;
/** The slap's share that is the speed carried on (the rest is the steep acceleration into the screen) */
const CARRY = (BEFORE_SLAP * (1 - SLAP_START)) / ((1 - BEFORE_SLAP) * SLAP_START);

/**
 * How far along the path (0…1, the Catmull-Rom's own parameter: a quarter per leg) the card is at `t`. Fluid: from
 * rest it gathers speed and never stops (x²(2 − x) – ends at full speed), the slap carries that speed on and then
 * accelerates hard (x⁴) – the only abrupt part is the smash itself.
 */
export const pathProgress = (t: number): number => {
  t = clamp01(t);
  if (t <= SLAP_START) {
    const x = t / SLAP_START;
    return BEFORE_SLAP * x * x * (2 - x);
  }
  const y = (t - SLAP_START) / (1 - SLAP_START);
  return BEFORE_SLAP + (1 - BEFORE_SLAP) * (CARRY * y + (1 - CARRY) * y ** 4);
};

/** The peel's peak (0…1 of the flight): the top edge is fully curled up off the page */
const PEEL_PEAK = 0.12;
/** The curl turns into the convex bow (the middle towards the reader) between these – already while it peels */
const CONVEX_FROM = 0.04;
const CONVEX_BY = 0.36;
/** The twist swells from here – also while it peels; peel, bow and twist overlap and blend */
const TWIST_FROM = 0.05;

/** 1 until the spin is over, then smoothly down to 0 early in the slap: when the paper becomes a stiff card */
const paperLeft = (t: number): number => 1 - easeInOutSine(clamp01((t - SLAP_START) / (0.4 * (1 - SLAP_START))));
/** 0 → 1 between `CONVEX_FROM` and `CONVEX_BY`: from the curl off the page to the convex bow */
const toConvex = (t: number): number => easeInOutSine(clamp01((t - CONVEX_FROM) / (CONVEX_BY - CONVEX_FROM)));

/**
 * How far the card bends at `t` (1 = curled by `CURL` towards the reader, negative = the other way): paper until the
 * spin is over, a card after it. It curls its top edge up as it peels off the page and, already while it does, turns
 * into a convex bow (its middle towards the reader, the edges back) that swells a little once through the rise and the
 * spin, and stiffens only once the spin is done. The bend's axis moves with it (`bendPivotAt`).
 */
export const bendAt = (t: number): number => {
  const peel = Math.sin((Math.PI / 2) * clamp01(t / PEEL_PEAK)) ** 2;
  // One slow swell of the bow over the way to the slap – no oscillation
  const bow = -(0.6 + 0.12 * Math.sin(Math.PI * clamp01((t - PEEL_PEAK) / (SLAP_START - PEEL_PEAK))));
  const c = toConvex(t);
  return ((1 - c) * peel + c * bow) * paperLeft(t);
};

/** Where the bend's axis lies at `t`, 0 = the bottom edge (the peel), 1 = the middle (the convex bow) */
export const bendPivotAt = (t: number): number => toConvex(t);

/** How far the card twists at `t` (1 = `TWIST` between its bottom and top edge): one slow twist, paper only */
export const twistAt = (t: number): number => {
  // One slow twist and back, swelling in while the card still peels and bows
  return Math.sin(Math.PI * clamp01((t - TWIST_FROM) / (SLAP_START - TWIST_FROM))) ** 2 * paperLeft(t);
};

/**
 * How long a flight from this page takes (ms): `near` when the page is about as close as the landing (the card barely
 * travels), `far` from three landing distances away and more – in between in proportion.
 */
export const flightMs = (start: CardStart, camera: Pick<PerspectiveCamera, "matrixWorld" | "projectionMatrix">, aspect: number): number => {
  const d = endDistance(camera.projectionMatrix, start.width, aspect);
  const camPos = new Vector3().setFromMatrixPosition(camera.matrixWorld);
  const away = start.position.distanceTo(camPos) / d;
  const share = clamp01((away - 1) / 2);
  return CARD_FLIGHT_MS.near + (CARD_FLIGHT_MS.far - CARD_FLIGHT_MS.near) * share;
};

const X_AXIS = new Vector3(1, 0, 0);
const Y_AXIS = new Vector3(0, 1, 0);
const Z_AXIS = new Vector3(0, 0, 1);

/**
 * The card's position and rotation at `t` (0…1 of the flight), in the scene. `camera` gives the end: its world
 * matrix (where the reader looks) and its projection (how far away the card fills the screen).
 */
export const cardPose = (
  start: CardStart,
  camera: Pick<PerspectiveCamera, "matrixWorld" | "projectionMatrix" | "quaternion">,
  aspect: number,
  t: number,
): { position: Vector3; rotation: Quaternion } => {
  t = clamp01(t);
  const d = endDistance(camera.projectionMatrix, start.width, aspect);
  const end = new Vector3(0, 0, -d).applyMatrix4(camera.matrixWorld);
  const right = new Vector3(1, 0, 0).transformDirection(camera.matrixWorld);
  // Up the page first (its +y, towards the top edge), only just off it – then up off the page
  const pageUp = new Vector3(0, 1, 0).applyQuaternion(start.rotation);
  const slide = start.position.clone().addScaledVector(pageUp, SLIDE * start.width).addScaledVector(start.normal, 0.08 * start.width);
  const up = slide.clone().addScaledVector(start.normal, LIFT * start.width);
  // The turn: on the way to a point straight behind the landing (so the slap comes straight at the reader), swung
  // out a little to the side
  const behind = new Vector3(0, 0, -BEHIND * d).applyMatrix4(camera.matrixWorld);
  const turn = up.clone().lerp(behind, 0.65).addScaledVector(right, SWING * start.width);
  const curve = new CatmullRomCurve3([start.position, slide, up, turn, end], false, "centripetal");
  const position = curve.getPoint(pathProgress(t));

  const { rise, spin } = phases(t);
  // Every turn rises and falls smoothly over the whole way to the slap (no phase starts or stops a turn abruptly)
  const facing = easeInOutSine(clamp01(t / SLAP_START));
  const peel = new Quaternion().setFromAxisAngle(X_AXIS, -0.35 * Math.sin(Math.PI * rise) ** 2);
  const turnY = new Quaternion().setFromAxisAngle(Y_AXIS, 2 * Math.PI * easeInOutSine(spin));
  const roll = new Quaternion().setFromAxisAngle(Z_AXIS, 0.28 * Math.sin(Math.PI * clamp01(t / SLAP_START)));
  const rotation = start.rotation.clone().slerp(camera.quaternion, facing).multiply(peel).multiply(turnY).multiply(roll);
  return { position, rotation };
};

/**
 * The impact at `ms` after the card reached the screen: `push` towards the reader (a share of the distance; it
 * bounces back) and `shake` sideways (card widths).
 */
export const impactAt = (ms: number): { push: number; shake: [number, number] } => {
  if (ms < 0 || ms >= IMPACT_MS) return { push: 0, shake: [0, 0] };
  const decay = Math.exp(-ms / 110);
  return {
    push: 0.09 * Math.exp(-ms / 70) * Math.cos(ms / 38),
    shake: [0.018 * decay * Math.sin(ms * 0.11), 0.014 * decay * Math.sin(ms * 0.17 + 1)],
  };
};

/** The card's back until the experience has one of its own: dark, a gold frame and a gold diamond (DESIGN.md's golds) */
const defaultBack = (): Texture => {
  const canvas = document.createElement("canvas");
  canvas.width = 256;
  canvas.height = 256;
  const g = canvas.getContext("2d");
  if (g) {
    g.fillStyle = "#14110d";
    g.fillRect(0, 0, 256, 256);
    const gold = g.createLinearGradient(0, 0, 256, 256);
    gold.addColorStop(0, "#f5e7c8");
    gold.addColorStop(0.32, "#7f6032");
    gold.addColorStop(0.57, "#d2ae5a");
    gold.addColorStop(1, "#f3cc94");
    g.strokeStyle = gold;
    g.lineWidth = 8;
    g.strokeRect(14, 14, 228, 228);
    g.beginPath();
    g.moveTo(128, 70);
    g.lineTo(178, 128);
    g.lineTo(128, 186);
    g.lineTo(78, 128);
    g.closePath();
    g.lineWidth = 5;
    g.stroke();
  }
  const texture = new CanvasTexture(canvas);
  texture.colorSpace = SRGBColorSpace;
  return texture;
};

/** The back's geometry: the front's positions (shared, so the bend reaches both), the other winding, mirrored u */
const backOf = (front: PlaneGeometry): PlaneGeometry => {
  const back = front.clone() as PlaneGeometry;
  back.setAttribute("position", front.getAttribute("position"));
  const index = back.getIndex();
  if (index) {
    const order = index.array.slice();
    for (let i = 0; i < order.length; i += 3) [order[i + 1], order[i + 2]] = [order[i + 2], order[i + 1]];
    back.setIndex(new BufferAttribute(order, 1));
  }
  const uv = back.getAttribute("uv") as BufferAttribute;
  for (let i = 0; i < uv.count; i++) uv.setX(i, 1 - uv.getX(i));
  return back;
};

/**
 * One flight: a two-sided card added to the scene, moved every frame by `update`, removed by `dispose`.
 * The back reads upright once the card has turned half way about its vertical axis. `front` is the printed picture
 * (the target), `reveal` the entry's own picture that takes its place when the card hits the screen (none: it stays).
 */
export class CardFlight {
  readonly object = new Group();
  private elapsed = 0;
  /** The flight's length (ms): given, or set from the page's distance on the first frame (`flightMs`) */
  private duration: number;

  get length(): number {
    return this.duration;
  }
  private start: CardStart;
  private textures: Texture[] = [];
  private geometry: PlaneGeometry;
  private flat: Float32Array;
  private frontMaterial: MeshBasicMaterial;
  private revealed = false;

  constructor(
    anchor: Matrix4,
    front: Texture,
    private aspect: number,
    back: Texture | null = null,
    duration: number | null = null,
    private reveal: Texture | null = null,
  ) {
    this.start = startFromAnchor(anchor);
    this.duration = duration ?? 0;
    const backTexture = back ?? defaultBack();
    this.textures = [front, backTexture, ...(reveal ? [reveal] : [])];
    this.geometry = new PlaneGeometry(1, aspect, 1, BEND_SEGMENTS);
    this.flat = (this.geometry.getAttribute("position").array as Float32Array).slice();
    // On top of the page and its entities: no depth test, drawn last
    const material = (map: Texture) => new MeshBasicMaterial({ map, side: FrontSide, depthTest: false, depthWrite: false, transparent: true });
    this.frontMaterial = material(front);
    const frontMesh = new Mesh(this.geometry, this.frontMaterial);
    const backMesh = new Mesh(backOf(this.geometry), material(backTexture));
    for (const mesh of [frontMesh, backMesh]) {
      mesh.renderOrder = 10_000;
      mesh.frustumCulled = false;
      this.object.add(mesh);
    }
    this.object.name = "marginalia-card";
  }

  /**
   * Advance by `delta` (s); `anchor` is the anchor's pose this frame (null: keep the last). False once the impact is
   * over. The card keeps the picture's width at the start, so it lands at the size `endDistance` planned for.
   */
  update(delta: number, anchor: Matrix4 | null, camera: PerspectiveCamera): boolean {
    if (anchor) {
      // Only the pose follows the page; the width stays, so the end distance does not jump with the tracking's scale
      this.start = { ...startFromAnchor(anchor), width: this.start.width };
    }
    if (!this.duration) this.duration = flightMs(this.start, camera, this.aspect);
    this.elapsed += delta * 1000;
    const t = Math.min(1, this.elapsed / this.duration);
    const { position, rotation } = cardPose(this.start, camera, this.aspect, t);
    const impact = impactAt(this.elapsed - this.duration);
    if (this.elapsed >= this.duration) {
      if (!this.revealed) {
        // The smash: the printed picture becomes the entry's own
        this.revealed = true;
        if (this.reveal) {
          this.frontMaterial.map = this.reveal;
          this.frontMaterial.needsUpdate = true;
        }
        if (typeof navigator !== "undefined") navigator.vibrate?.([40, 30, 60]);
      }
      const towards = new Vector3().setFromMatrixPosition(camera.matrixWorld).sub(position);
      position.addScaledVector(towards, impact.push);
      const right = new Vector3(1, 0, 0).applyQuaternion(rotation);
      const up = new Vector3(0, 1, 0).applyQuaternion(rotation);
      position.addScaledVector(right, impact.shake[0] * this.start.width).addScaledVector(up, impact.shake[1] * this.start.width);
    }
    this.bend(bendAt(t), twistAt(t), bendPivotAt(t));
    this.object.position.copy(position);
    this.object.quaternion.copy(rotation);
    this.object.scale.setScalar(this.start.width);
    return this.elapsed < this.duration + IMPACT_MS;
  }

  /** Curl the card's top edge up off the page by `amount` (0 = flat): a cylinder whose axis lies along the bottom edge */
  private bend(amount: number, twist = 0, pivot = 0): void {
    const positions = this.geometry.getAttribute("position") as BufferAttribute;
    const height = this.aspect;
    const bottom = -height / 2;
    // The bend's axis: from the bottom edge (the peel) to the middle (the bow)
    const axis = bottom + (pivot * height) / 2;
    const k = (amount * CURL) / height; // curvature: an edge turns by up to `amount · CURL`
    for (let i = 0; i < positions.count; i++) {
      const x = this.flat[i * 3];
      const y = this.flat[i * 3 + 1];
      const along = y - axis;
      const by = Math.abs(k) < 1e-5 ? y : axis + Math.sin(k * along) / k;
      const bz = Math.abs(k) < 1e-5 ? 0 : (1 - Math.cos(k * along)) / k;
      // The twist: each row turns about the card's vertical axis, more towards the top
      const angle = twist * TWIST * ((y - bottom) / height - 0.5);
      positions.setXYZ(i, x * Math.cos(angle), by, bz + x * Math.sin(angle));
    }
    positions.needsUpdate = true;
  }

  dispose(): void {
    this.object.removeFromParent();
    this.object.traverse(node => {
      if (node instanceof Mesh) {
        (node.material as MeshBasicMaterial).dispose();
        node.geometry.dispose();
      }
    });
    // The flight owns its textures (the pictures are loaded for it)
    this.textures.forEach(texture => texture.dispose());
  }
}
