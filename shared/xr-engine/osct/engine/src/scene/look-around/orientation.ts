import { Euler, Quaternion, Vector3 } from "three";

const DEG = Math.PI / 180;
const Z_AXIS = new Vector3(0, 0, 1);
/** The camera looks out of the back of the device: −90° about x */
const BACK_CAMERA = new Quaternion(-Math.SQRT1_2, 0, 0, Math.SQRT1_2);

/**
 * The camera's orientation in the world (y up, −z = the direction alpha counts from) for device orientation
 * angles in degrees and the screen's rotation in degrees – the maths of three.js' former
 * DeviceOrientationControls.
 */
export const deviceQuaternion = (alpha: number, beta: number, gamma: number, screenAngle: number, out = new Quaternion()): Quaternion => {
  out.setFromEuler(new Euler(beta * DEG, alpha * DEG, -gamma * DEG, "YXZ"));
  out.multiply(BACK_CAMERA);
  out.multiply(new Quaternion().setFromAxisAngle(Z_AXIS, -screenAngle * DEG));
  return out;
};

interface PermissionRequesting {
  requestPermission?: () => Promise<"granted" | "denied">;
}

/**
 * The phone's orientation from the gyroscope (`deviceorientation`, 3DoF): rotation only, no position. iOS asks for
 * motion permission, which needs a tap – the first tap anywhere asks (`listen()` installs that).
 */
export class DeviceOrientation {
  readonly quaternion = new Quaternion();
  /** An orientation event with angles has arrived (desktop browsers never send one) */
  available = false;
  private listening = false;
  private asked = false;

  listen(): void {
    if (this.listening) return;
    this.listening = true;
    window.addEventListener("deviceorientation", this.onOrientation);
    if (this.needsPermission()) document.addEventListener("click", this.askPermission, { capture: true });
  }

  stop(): void {
    this.listening = false;
    this.available = false;
    window.removeEventListener("deviceorientation", this.onOrientation);
    document.removeEventListener("click", this.askPermission, { capture: true });
  }

  private needsPermission(): boolean {
    const api = window.DeviceOrientationEvent as unknown as PermissionRequesting | undefined;
    return !this.asked && typeof api?.requestPermission === "function";
  }

  /** Inside a tap (user activation), as iOS requires */
  private askPermission = (): void => {
    if (this.asked) return;
    this.asked = true;
    document.removeEventListener("click", this.askPermission, { capture: true });
    const api = window.DeviceOrientationEvent as unknown as PermissionRequesting;
    api.requestPermission?.().then(
      state => console.info(`[look-around] Motion permission ${state}`),
      error => console.warn("[look-around] Motion permission failed:", error),
    );
  };

  /** Set the orientation by hand (dev: desktop browsers send no orientation events) */
  set(alpha: number, beta: number, gamma: number): void {
    deviceQuaternion(alpha, beta, gamma, 0, this.quaternion);
  }

  private onOrientation = (event: DeviceOrientationEvent): void => {
    if (event.alpha === null || event.beta === null || event.gamma === null) return;
    const angle = screen.orientation?.angle ?? (window as unknown as { orientation?: number }).orientation ?? 0;
    deviceQuaternion(event.alpha, event.beta, event.gamma, angle, this.quaternion);
    this.available = true;
  };
}
