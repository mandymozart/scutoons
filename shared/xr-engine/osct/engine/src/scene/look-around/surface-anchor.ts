import { type Matrix4, type Quaternion, Vector3 } from "three";

const DEG = Math.PI / 180;

/** Turn about y that brings the front (−z) to face a direction (its horizontal part) */
export const yawTowards = (direction: Vector3): number => Math.atan2(-direction.x, -direction.z);

/** Shortest step from angle `from` to `to`, by `amount` (0…1) */
export const lerpAngle = (from: number, to: number, amount: number): number => {
  const difference = Math.atan2(Math.sin(to - from), Math.cos(to - from));
  return from + difference * amount;
};

/**
 * The surfaces' window in degrees: the page's angular radius (its width `size` seen from `distance`, the page being
 * a bit larger than one target) plus a margin; the soft edge ends `softness` degrees further out.
 */
export const windowRadii = (size: number, distance: number, margin: number, softness: number): { inner: number; outer: number } => {
  const page = distance > 0 ? Math.atan((size * 0.9) / distance) / DEG : 20;
  const inner = Math.min(45, Math.max(14, page + margin));
  return { inner, outer: inner + softness };
};

/** Smoothing per update while the surfaces are seen (tracking jitter must not shake the world) */
const DIRECTION_SMOOTHING = 0.2;
const YAW_SMOOTHING = 0.05;
const WINDOW_SMOOTHING = 0.2;

/**
 * Where the found surfaces are in the world, from the found pages' anchors (relative to the camera) and the phone's orientation:
 * its direction (the window around it), the world's turn (its front faces the surfaces) and how long ago it was seen.
 * The first find places it at once; later finds move it smoothly, which also corrects the gyroscope's drift.
 */
export class SurfaceAnchor {
  /** The surfaces' direction in the world while known */
  direction: Vector3 | null = null;
  /** The world's turn about y: its front (−z) faces the surfaces */
  yaw = 0;
  /** The window around the surfaces, in degrees (inner = clear, outer = end of the soft edge) */
  readonly window = { inner: 20, outer: 36 };
  /** Seconds since a page was last seen */
  sinceSeen = Infinity;

  private centre = new Vector3();
  private scale = new Vector3();
  private seen = new Vector3();
  private flat = new Vector3();

  see(anchors: readonly Matrix4[], orientation: Quaternion, margin: number, softness: number): void {
    const centre = this.centre.set(0, 0, 0);
    let size = 0;
    anchors.forEach(matrix => {
      centre.add(this.seen.setFromMatrixPosition(matrix));
      size = Math.max(size, this.scale.setFromMatrixScale(matrix).x);
    });
    centre.divideScalar(anchors.length);
    const distance = centre.length();
    const direction = this.seen.copy(centre).normalize().applyQuaternion(orientation);
    const first = this.direction === null;
    this.direction = first ? direction.clone() : this.direction!.lerp(direction, DIRECTION_SMOOTHING).normalize();

    this.flat.set(this.direction.x, 0, this.direction.z);
    if (this.flat.lengthSq() > 1e-4) {
      const yaw = yawTowards(this.flat);
      this.yaw = first ? yaw : lerpAngle(this.yaw, yaw, YAW_SMOOTHING);
    }
    const radii = windowRadii(size, distance, margin, softness);
    this.window.inner += (radii.inner - this.window.inner) * WINDOW_SMOOTHING;
    this.window.outer += (radii.outer - this.window.outer) * WINDOW_SMOOTHING;
    this.sinceSeen = 0;
  }

  /** Time passes without a page in view */
  unseen(delta: number): void {
    this.sinceSeen += delta;
  }

  forget(): void {
    this.direction = null;
    this.sinceSeen = Infinity;
  }
}
