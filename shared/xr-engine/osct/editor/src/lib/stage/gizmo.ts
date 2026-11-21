import { MathUtils, type Object3D, Quaternion, Vector3 } from "three";
import { TransformControls } from "three/examples/jsm/controls/TransformControls.js";
import type { SceneInspection, Vector3Data } from "@engine/types";
import { outline, SELECTION_COLOR, type Outline } from "./shading";

/**
 * The transform gizmo of the editor's stage: three.js `TransformControls` on the selected node's object (the
 * placement group the engine builds per node), in the node's own space – a drag changes the node's position,
 * rotation or scale **relative to its parent**, exactly the values the Inspector shows. Modes move / rotate /
 * scale (W / E / R, the toolbar). Snapping (the toolbar's *grid & snap*, Shift or Ctrl held turns it around) is the
 * snapper's (`snap-scene.ts`): a move snaps to the grid, the axes, other objects and outlines, a rotation's angle to
 * whole steps of degrees, a scale to whole steps. The hover is the stage's (it follows the pick mode). The tracker
 * ignores pointer gestures while an axis is hovered or dragged (`busy`), and a tap right after a drag is no selection
 * (`recentlyDragged`).
 */

export type GizmoMode = "translate" | "rotate" | "scale";
export const GIZMO_MODES: readonly GizmoMode[] = ["translate", "rotate", "scale"];
export const GIZMO_KEYS: Record<string, GizmoMode> = { w: "translate", e: "rotate", r: "scale" };
/**
 * The tool in hand (#308): **select**, a plain pointer – a click picks, the selection is outlined, no gizmo – or one of
 * the gizmo's modes – or **surface** (#314, the Surface mode): the scene's surfaces outlined, a click selects a surface
 * (no target, no node), the selected one highlighted (in a free scene with its own gizmo). Q / W / E / R / S.
 */
export type StageTool = "select" | GizmoMode | "surface";
export const STAGE_TOOLS: readonly StageTool[] = ["select", ...GIZMO_MODES, "surface"];
export const TOOL_KEYS: Record<string, StageTool> = { q: "select", ...GIZMO_KEYS, s: "surface" };
/** The tool is one of the gizmo's modes (the node's handles show) */
export const isGizmoTool = (tool: StageTool): tool is GizmoMode => (GIZMO_MODES as readonly string[]).includes(tool);
/** A click this soon after a drag ended is the drag's own click, not a tap */
const DRAG_CLICK_MS = 300;

export interface GizmoPlacement {
  position: Vector3Data;
  rotation: Vector3Data;
  scale: number | Vector3Data;
}

const round = (value: number) => +value.toFixed(4);

/** The node's placement as the content stores it: position, rotation in degrees, an even scale as one number */
export const placementOf = (object: Object3D): GizmoPlacement => {
  const position: Vector3Data = [round(object.position.x), round(object.position.y), round(object.position.z)];
  const rotation: Vector3Data = [
    round(MathUtils.radToDeg(object.rotation.x)),
    round(MathUtils.radToDeg(object.rotation.y)),
    round(MathUtils.radToDeg(object.rotation.z)),
  ];
  const s: Vector3Data = [round(object.scale.x), round(object.scale.y), round(object.scale.z)];
  return { position, rotation, scale: s[0] === s[1] && s[1] === s[2] ? s[0] : s };
};

export interface GizmoSelection {
  targetId: string;
  path: number[];
}

/** What the gizmo asks while it snaps (`SceneSnapper`) */
export interface GizmoSnapper {
  active(): boolean;
  angle(degrees: number): number;
  scale(value: number): number;
  beginMove(object: Object3D, axis: string | null): void;
  move(object: Object3D): void;
  end(): void;
}

/**
 * A rotation dragged from `start` to `now` (the node's own space) with its angle snapped: the turn between them, about
 * its own axis, by the snapped angle
 */
export const snappedRotation = (start: Quaternion, now: Quaternion, snapDegrees: (degrees: number) => number): Quaternion => {
  const delta = start.clone().invert().multiply(now).normalize();
  if (delta.w < 0) delta.set(-delta.x, -delta.y, -delta.z, -delta.w);
  const half = Math.acos(Math.min(1, delta.w));
  const sin = Math.sin(half);
  if (sin < 1e-9) return start.clone();
  const axis = new Vector3(delta.x / sin, delta.y / sin, delta.z / sin);
  const angle = MathUtils.degToRad(snapDegrees(MathUtils.radToDeg(2 * half)));
  return start.clone().multiply(new Quaternion().setFromAxisAngle(axis, angle)).normalize();
};

export class Gizmo {
  private controls: TransformControls;
  private helper: Object3D;
  private selectionOutline: Outline | null = null;
  private selection: GizmoSelection | null = null;
  private attached: Object3D | null = null;
  private dragging = false;
  private dragEndedAt = 0;
  /** The node's rotation when the drag started (a rotation snaps its angle from here) */
  private startQuaternion = new Quaternion();
  private offFrame: () => void;

  constructor(
    private inspection: SceneInspection,
    /** A drag changed the selected node's placement */
    private onchange: (selection: GizmoSelection, placement: GizmoPlacement) => void,
    private snapper: GizmoSnapper | null = null,
  ) {
    const { camera, domElement, scene } = inspection;
    this.controls = new TransformControls(camera, domElement);
    this.controls.setSpace("local");
    this.controls.size = 0.8;
    this.helper = this.controls.getHelper();
    this.helper.name = "editor-gizmo";
    scene.add(this.helper);
    this.controls.addEventListener("dragging-changed", event => {
      this.dragging = !!event.value;
      if (this.dragging && this.attached) {
        this.startQuaternion.copy(this.attached.quaternion);
        if (this.mode === "translate") this.snapper?.beginMove(this.attached, this.controls.axis);
      }
      if (!this.dragging) {
        this.dragEndedAt = performance.now();
        this.snapper?.end();
      }
    });
    this.controls.addEventListener("objectChange", () => {
      if (!this.selection || !this.attached) return;
      this.snapDrag(this.attached);
      this.onchange(this.selection, placementOf(this.attached));
    });
    // A rebuilt node (the scene built the scene again) is attached and outlined again
    this.offFrame = inspection.onFrame(() => {
      if (this.selection && this.attached && !this.attached.parent) this.sync();
    });
  }

  get mode(): GizmoMode {
    return this.controls.mode as GizmoMode;
  }

  set mode(mode: GizmoMode) {
    this.controls.setMode(mode);
  }

  /** The drag moved the node: snapped by the mode while snapping is on (a move asks the snapper even then: it clears its guides) */
  private snapDrag(object: Object3D): void {
    const snapper = this.snapper;
    if (!snapper) return;
    if (this.mode === "translate") return snapper.move(object);
    if (!snapper.active()) return;
    if (this.mode === "rotate") {
      object.quaternion.copy(snappedRotation(this.startQuaternion, object.quaternion, degrees => snapper.angle(degrees)));
    } else {
      object.scale.set(snapper.scale(object.scale.x), snapper.scale(object.scale.y), snapper.scale(object.scale.z));
    }
  }

  /** An axis is hovered or dragged: the gesture belongs to the gizmo, not to the stage's orbit */
  get busy(): boolean {
    return this.dragging || !!this.controls.axis;
  }

  recentlyDragged(): boolean {
    return this.dragging || performance.now() - this.dragEndedAt < DRAG_CLICK_MS;
  }

  /** The handles show (a gizmo mode); off (the select tool) the selected node is only outlined */
  get handles(): boolean {
    return this.showHandles;
  }

  set handles(on: boolean) {
    if (on === this.showHandles) return;
    this.showHandles = on;
    if (on && this.attached) this.controls.attach(this.attached);
    else this.controls.detach();
  }

  private showHandles = true;

  /** Put the gizmo on a node (null = none) */
  select(selection: GizmoSelection | null): void {
    this.selection = selection;
    this.sync();
  }

  /** Attach the selected node's current object (the scene may have rebuilt it) and outline it */
  sync(): void {
    const object = this.selection ? this.inspection.nodeObject(this.selection.targetId, this.selection.path) : null;
    if (object === this.attached && (object && this.showHandles ? this.controls.object === object : true)) return;
    this.attached = object;
    this.selectionOutline?.remove();
    this.selectionOutline = object ? outline(object, SELECTION_COLOR) : null;
    if (object && this.showHandles) this.controls.attach(object);
    else this.controls.detach();
  }

  /** The selection's outline drawn again (a node's meshes changed, the shading swapped materials) */
  refreshOutline(): void {
    this.selectionOutline?.remove();
    this.selectionOutline = this.attached ? outline(this.attached, SELECTION_COLOR) : null;
  }

  dispose(): void {
    this.offFrame();
    this.selectionOutline?.remove();
    this.selectionOutline = null;
    this.controls.detach();
    this.helper.removeFromParent();
    this.controls.dispose();
  }
}
