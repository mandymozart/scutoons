import {
  AddEquation, CustomBlending, NormalBlending, OneFactor, OneMinusDstAlphaFactor,
  BoxGeometry, CanvasTexture, EquirectangularReflectionMapping, Group, Light, LineBasicMaterial, LineSegments, BufferGeometry, Float32BufferAttribute, type Material, Matrix4, Mesh, MeshBasicMaterial, MeshStandardMaterial, type Object3D,
  PlaneGeometry, type Raycaster, RepeatWrapping, type Scene, SphereGeometry, SRGBColorSpace, type Texture, TextureLoader, UVMapping, Vector3,
} from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { RGBELoader } from "three/examples/jsm/loaders/RGBELoader.js";
import { BACKDROP_LAYER } from "@engine/scene/view";
import type { StageScene, StageGrids, StageSurface, StageTarget, StageView } from "./mock-tracker";
import { frameFor, placeHeight } from "./placement";
import { frameBox, gridLines, originGizmo, subdivisionLines, ORIGIN_SPOTS, originPoint, type OriginFrame } from "./origin";
import { DEFAULT_FRAME, fitImage, hitSurfaces, surfaceFrame, surfaceRects, toStage, type SurfaceFrame, type SurfaceReference, type V3 } from "./surfaces";
import { linoleumTexture } from "./linoleum";
import { oakTexture } from "./oak";
import { plywoodEdgeTexture } from "./plywood";
import { applyShading, ownMaterial } from "./shading";
import type { HdriId, ShadingMode, TopId } from "./preview";
import { stageLook, type StageLook } from "./sky";

/**
 * A plane sits a hair below the one above it, so nothing flickers: the entities' plane is the target's (z = 0, as
 * the tracker reports it), the printed image lies one layer under it, the reference image another, the board's face
 * under that. Half a millimetre is invisible at a reading distance and enough for the depth buffer.
 */
const LAYER = 0.0005;
/**
 * Colours. The Eiermann desk's top is a screen-printed plywood board (Siebdruckplatte): Poly Haven's plywood texture
 * tinted to the dark phenolic brown on the faces, roughness from its map; its cut edges show the plywood's plies
 * (`plywood.ts`, drawn in code), untinted and matt. The linoleum top (#226, once Schöpfer's – #187) is blue-grey
 * linoleum (`linoleum.ts`) on an oak-faced plywood board: its cut edges show the same plies tinted to oak, its underside
 * the oak veneer (`oak.ts`) – all drawn in code, matt. A surface is a plain board, its face paper.
 */
const DESK_TINT = 0x3b2416;
const DESK_EDGE_TINT = 0xc8c8c8;
const OAK_EDGE_TINT = 0xe8c290;
const DESK_EDGE_ROUGHNESS = 0.8;
const LINOLEUM_ROUGHNESS = 0.85;
const OAK_ROUGHNESS = 0.65;
/**
 * The plies in one stack of the edges' texture: both tops' boards are 18 mm plywood – 13 plies of about 1.4 mm, as the
 * trade's 18 mm birch plywood. Each board's edge is one stack (the model's UVs)
 */
const DESK_EDGE_PLIES = 13;
const PAPER_COLOR = 0xf3efe6;
const PAPER_EDGE_COLOR = 0xe6e1d6;
/**
 * The desk (#226): the Eiermann frame, 72 cm high, with two tops of the same 1.8 × 0.8 m and the same surface height,
 * one model with a mesh per top (`TOP_NODES`) – loaded once, the top not chosen hidden, so a switch moves nothing
 * (our own low-poly model, built by `editor/tools/desk-eiermann.py` – `editor/static/models/README.md`). Its origin is
 * the centre of the tops' upper surface; the materials with a drawn look become the stage's own – the plywood top's
 * `top` (the screen-printed plywood, see below) and `edge` (its plies), the linoleum top's `linoleum`, `oak_edge` (the
 * plies in oak) and `oak` – while `steel` stays as exported. glTF is Y up: the model is turned so its up is the page's +z.
 */
const DESK_MODEL = "models/desk-eiermann.glb";
const TOP_NODES: Readonly<Record<TopId, string>> = { plywood: "TopEiermann", linoleum: "TopLinoleum" };
/** The desk texture's files and how often it repeats across the top (the edges have their own texture and UVs) */
const DESK_DIFFUSE = "textures/plywood_diff_1k.jpg";
const DESK_ROUGHNESS = "textures/plywood_rough_1k.jpg";
const DESK_REPEAT = [3, 2] as const;
/**
 * The HDRIs are not tone mapped (the reader's videos and images must keep their colours), so each is shown and used
 * as light at an exposure that keeps the page white, not blown out; the scene's own lights step back meanwhile.
 */
const HDRI_EXPOSURE: Record<string, number> = { ferndale_studio_04: 0.9 };
const HDRI_EXPOSURE_DEFAULT = 0.4;
const LIGHTS_WITH_HDRI = 0.35;
/** The sky sphere's radius (metres) – far beyond the table, near enough for the depth buffer */
const SKY_RADIUS = 30;
/**
 * The sky sphere behind the environment's world (#185): the world's pass is drawn first and leaves its camera window
 * transparent (alpha 0, premultiplied soft edges), so the sphere adds itself only where the frame is not yet covered –
 * `(1 − destination alpha) · sky + frame` – and fills the window as a stand-in for the reader's camera picture. Off: the
 * sphere draws normally (the HDRI as the sky).
 */
export const behindWorld = (material: Material, on: boolean): void => {
  material.blending = on ? CustomBlending : NormalBlending;
  material.blendEquation = AddEquation;
  material.blendSrc = OneMinusDstAlphaFactor;
  material.blendDst = OneFactor;
  material.blendSrcAlpha = OneMinusDstAlphaFactor;
  material.blendDstAlpha = OneFactor;
};
/** The ghost's corner handle (mm) */
const GHOST_HANDLE = 8;
const GRID_COLOR = 0x2d7ff9;
const GRID_ZERO_COLOR = 0xe5484d;
/** The vertical axis: the height's line through the origin, in the 3D tools' colour for the up axis, this tall (mm) */
const GRID_AXIS_COLOR = 0x30a46c;
const GRID_AXIS_MM = 150;
/** A snap guide: the line a drag snapped to, across the scene and this far beyond it (mm) */
const GUIDE_COLOR = 0xd23cf0;
const GUIDE_OVERHANG_MM = 20;

/** The scene's axes at its origin (#314): x red, y (up) green, z (towards the reader) blue, as 3D tools draw them (mm long) */
const AXIS_X_COLOR = 0xe5484d;
const AXIS_Z_COLOR = 0x2d7ff9;
const SCENE_AXIS_MM = 100;
/** The ring round the scene's origin (the origin gizmo, #314) */
const ORIGIN_RING_COLOR = 0xffffff;
/**
 * The scene's floor grid (#319, after the SCUTOONS editor's): a transparent net on the table (the scene's y = 0), every
 * `floorMm`, every tenth line a little stronger, this far around the origin (mm); the vertical grid the same net standing
 * at the scene's z = `wallZ`, this high. Grey – the editor is monochrome (RULES #38).
 */
const FLOOR_GRID_COLOR = 0xb4b4b4;
const FLOOR_GRID_EXTENT_MM = 1500;
const WALL_GRID_HEIGHT_MM = 2500;
const FLOOR_GRID_MAX_LINES = 400;
/** A surface is a board this thick (m), lying behind its face */
const BOARD_THICKNESS = 0.0015;
/** The face of a surface lies this far under its place (m): the reference, the prints and the entities above it */
const FACE = -3 * LAYER;
/**
 * The surfaces' outlines (#314, the Surface mode): all surfaces grey, the hovered one lighter, the highlighted (selected)
 * one white with a black line inside – the editor is monochrome (RULES #38)
 */
const SURFACE_OUTLINE_COLOR = 0x8c8c8c;
const SURFACE_HOVER_COLOR = 0xc8c8c8;
const SURFACE_HIGHLIGHT_COLOR = 0xffffff;
const SURFACE_HIGHLIGHT_INNER_COLOR = 0x111111;
/**
 * The target marks (#323, after the #321 contact sheets): a frame around each target's printed place and its id on a
 * label at the frame's top left – green and solid for a target in the experience, orange and dashed for a switched-off
 * one (colour and line both, so it reads without the colour too). An explicit exception to the monochrome editor
 * (RULES #38), as the contact sheets – Tilman, 2026-11-19: "Green and orange and dashed"
 */
const TARGET_ON_COLOR = 0x2fbf4f;
const TARGET_OFF_COLOR = 0xff8a1f;
const TARGET_ON_CSS = "#2fbf4f";
const TARGET_OFF_CSS = "#ff8a1f";
/** A dash and its gap along a switched-off target's frame (mm) */
const DASH_MM = 3;
const GAP_MM = 2;
/** The label's height on the surface (mm) */
const LABEL_MM = 5;

/** A line from (ax, ay) to (bx, by) as dashes: [x1, y1, x2, y2] each */
export const dashes = (ax: number, ay: number, bx: number, by: number, dash = DASH_MM, gap = GAP_MM): number[][] => {
  const length = Math.hypot(bx - ax, by - ay);
  if (!(length > 0)) return [];
  const out: number[][] = [];
  for (let at = 0; at < length; at += dash + gap) {
    const end = Math.min(length, at + dash);
    out.push([ax + ((bx - ax) * at) / length, ay + ((by - ay) * at) / length, ax + ((bx - ax) * end) / length, ay + ((by - ay) * end) / length]);
  }
  return out;
};

/** A target mark's label: its id, dark on the mark's colour; null where there is no 2D canvas (the tests) */
const labelTexture = (text: string, color: string): CanvasTexture | null => {
  if (typeof document === "undefined") return null;
  const canvas = document.createElement("canvas");
  let context: CanvasRenderingContext2D | null = null;
  try {
    context = canvas.getContext("2d");
  } catch {
    return null;
  }
  if (!context) return null;
  const font = "600 40px system-ui, sans-serif";
  context.font = font;
  canvas.width = Math.ceil(context.measureText(text).width) + 24;
  canvas.height = 56;
  context.font = font;
  context.fillStyle = color;
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = "#111111";
  context.textBaseline = "middle";
  context.fillText(text, 12, canvas.height / 2 + 2);
  const texture = new CanvasTexture(canvas);
  texture.colorSpace = SRGBColorSpace;
  texture.anisotropy = 4;
  return texture;
};

/** Lays a plane (its face +z) on a surface: centred at (x, y) mm of the surface, `lift` m off its face, turned with it */
const layOnSurface = (object: Object3D, frame: SurfaceFrame, x: number, y: number, lift: number): void => {
  const at = toStage(frame, x, y, lift * 1000);
  object.position.set(at[0] / 1000, at[1] / 1000, at[2] / 1000);
  object.quaternion.setFromRotationMatrix(new Matrix4().makeBasis(vec(frame.right), vec([-frame.down[0], -frame.down[1], -frame.down[2]]), vec(frame.normal)));
};
const vec = (v: V3): Vector3 => new Vector3(v[0], v[1], v[2]);

/** An origin's point on a surface's own frame (mm from its top left corner): a corner, an edge's centre or its middle */
const spotOf = (origin: keyof typeof ORIGIN_SPOTS, frame: Pick<SurfaceFrame, "widthMm" | "heightMm">): { x: number; y: number } => {
  const { col, row } = ORIGIN_SPOTS[origin];
  return { x: (col * frame.widthMm) / 2, y: (row * frame.heightMm) / 2 };
};

/** Frees and removes a group's lines (the grid, the guides) */
const clearLines = (group: Group): void => {
  group.children.forEach(child => {
    if (child instanceof LineSegments) {
      child.geometry.dispose();
      (child.material as Material).dispose();
    }
  });
  group.clear();
};

/**
 * Adds line segments (pairs of points, stage metres) to a group, drawn over what lies under them without hiding it;
 * `throughAll` draws them through everything (the guides, which an entity must not cover)
 */
const addLines = (group: Group, positions: number[], color: number, opacity: number, throughAll = false): void => {
  if (!positions.length) return;
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new Float32BufferAttribute(positions, 3));
  const lines = new LineSegments(geometry, new LineBasicMaterial({ color, transparent: true, opacity, depthWrite: false, depthTest: !throughAll }));
  if (throughAll) lines.renderOrder = 1;
  group.add(lines);
};

/** The Eiermann top's plywood faces, repeated over the top */
interface DeskTextures {
  map: Texture;
  roughnessMap: Texture;
}

/**
 * The mock stage: what the camera would show – a desk, the open scene's surfaces as plain boards at their places (each
 * with its reference image, else paper), the targets' printed images where they lie (or in a row), the grids, all in
 * the same three.js scene as the entities, moved as one group by the tracker's camera
 * (the three.js camera stays at the origin, as in AR). The sky and the light (`sky.ts`, #185): the chosen HDRI while the
 * shading is rendered, the scene's own lights otherwise – and while the experience's environment is drawn under the stage,
 * its world as the sky and the scene's own lights, as in the reader. The desk keeps its own materials under every viewport shading
 * (`shading.ts` applies to the entities), like the gizmo and the outlines.
 */
export class MockStage implements StageView {
  private group = new Group();
  /** The desk (the model, once loaded): its origin is the tops' surface, placed at the table height */
  private table = new Group();
  /** The chosen top: its mesh shows, the other's is hidden */
  private topId: TopId = "plywood";
  private model: Group | null = null;
  private topMaterial = new MeshStandardMaterial({ color: DESK_TINT, roughness: 0.5 });
  private edgeMaterial = new MeshStandardMaterial({ color: DESK_EDGE_TINT, roughness: DESK_EDGE_ROUGHNESS });
  /** The linoleum top's surfaces: their drawn textures are put on when the top is first shown */
  private linoleumMaterial = new MeshStandardMaterial({ roughness: LINOLEUM_ROUGHNESS });
  private oakEdgeMaterial = new MeshStandardMaterial({ color: OAK_EDGE_TINT, roughness: DESK_EDGE_ROUGHNESS });
  private oakMaterial = new MeshStandardMaterial({ roughness: OAK_ROUGHNESS });
  /** The desk's materials that are the stage's own, by the model's material names (shared, disposed apart) */
  private deskMaterials = new Map<string, MeshStandardMaterial>([
    ["top", this.topMaterial],
    ["edge", this.edgeMaterial],
    ["linoleum", this.linoleumMaterial],
    ["oak_edge", this.oakEdgeMaterial],
    ["oak", this.oakMaterial],
  ]);
  /** The tops whose look their materials show (each put on once, when the top is first chosen) */
  private looks = new Set<TopId>();
  private disposed = false;
  private boards = new Group();
  private pages = new Group();
  private textures: Texture[] = [];
  private loader = new TextureLoader();
  private layoutRun = 0;
  private desk: DeskTextures | null = null;
  /** The drawn textures, each drawn once on first use: the boards' plies (both tops), the linoleum and the oak */
  private plies: Texture | null = null;
  private linoleum: Texture | null = null;
  private oak: Texture | null = null;
  /** The scene's surfaces in the stage frame (mm, #314): the prints lie on them, the hits find them */
  private frames: SurfaceFrame[] = [DEFAULT_FRAME];
  /** The scene's axes at its origin, its floor grid and its vertical grid (#319) */
  private sceneGridGroup = new Group();
  /** The surfaces outlined or highlighted (the Surface mode) */
  private marksGroup = new Group();
  private marks: { outlined: string[]; highlighted: string | null } = { outlined: [], highlighted: null };
  /** The target marks (#323): each target's place framed with its id, switched on in the grid menu */
  private targetMarksGroup = new Group();
  private targetMarks = false;
  private targetMarksQueued = false;
  /** The marks' labels by target id and state, made once per layout */
  private labelTextures = new Map<string, CanvasTexture>();
  /** Where the table's top lies (m, the stage frame's z): the scene's y = 0, under the flat boards */
  private tableTop = FACE - BOARD_THICKNESS;
  private ghostGroup = new Group();
  private gridGroup = new Group();
  private grid: StageGrids = { on: false, mm: 10, origin: "top-left", axis: true, surfaces: {}, floor: true, floorMm: 50, wall: false, wallZ: -500 };
  /** The snap guides of the drag going on */
  private guideGroup = new Group();
  private ghostTextures = new Map<string, Texture>();
  /** Every printed image's height per width by its source, once loaded (the ghost and a moved print know it at once) */
  private aspects = new Map<string, number>();
  private hdris = new Map<string, Texture>();
  private wantedHdri: string | null = null;
  /** The sky: the HDRI on a sphere inside the stage's group, so it turns with the stage by construction */
  private sky: Mesh<SphereGeometry, MeshBasicMaterial> | null = null;
  private shading: ShadingMode = "rendered";
  private hdri: HdriId | null = null;
  /** The experience's environment is drawn under the stage (the engine's look-around world): it replaces the HDRI (#185) */
  private world = false;

  /** The environment's world is drawn under the stage (true) or not: the HDRI steps aside – sky and light – while it is */
  setWorld(world: boolean): void {
    if (this.world === world) return;
    this.world = world;
    this.setHdri(this.hdri);
  }

  /** Which sky and which light now (`sky.ts`) */
  private look(): StageLook {
    return stageLook({ shading: this.shading, hdri: this.hdri, world: this.world });
  }

  /** The viewport's shading: the sky and the HDRI's light belong to "rendered" only; wireframe and solid reach the boards and the desk too (#137) */
  setShading(mode: ShadingMode): void {
    if (this.shading === mode) return;
    this.shading = mode;
    this.setHdri(this.hdri);
    this.shadeStage();
  }

  /** The current shading on the desk, the boards and the pages – not the ghost, the grid or the sky */
  private shadeStage(): void {
    applyShading([this.table, this.boards, this.pages], this.shading);
  }
  /** The scene's own lights at their intensity before the HDRI dimmed them */
  private dimmed = new Map<Light, number>();

  /** `assetUrl`: where the editor's static files live (`hdri/…`, `textures/…`) */
  constructor(private scene: Scene, private assetUrl: (path: string) => string = path => `/${path}`) {
    this.group.name = "mock-stage";
    this.group.matrixAutoUpdate = false;
    this.table.name = "mock-desk";
    this.table.position.z = this.tableTop - LAYER;
    this.ghostGroup.name = "mock-ghost";
    this.gridGroup.name = "mock-grid";
    this.guideGroup.name = "mock-guides";
    this.sceneGridGroup.name = "mock-scene-grid";
    this.marksGroup.name = "mock-surface-marks";
    this.targetMarksGroup.name = "mock-target-marks";
    this.group.add(this.table, this.boards, this.pages, this.gridGroup, this.sceneGridGroup, this.marksGroup, this.targetMarksGroup, this.guideGroup, this.ghostGroup);
    scene.add(this.group);
    this.loadDesk();
  }

  /** The desk's top (#226): only the top switches – the frame, the size and the height stay, so nothing else moves */
  setTop(top: TopId): void {
    if (this.topId === top && this.looks.has(top)) return;
    this.topId = top;
    this.showTop();
    this.applyTopLook(top);
  }

  /** The desk's model, loaded once into the desk's group: the frame and both tops, the chosen one shown */
  private loadDesk(): void {
    new GLTFLoader().load(this.assetUrl(DESK_MODEL), gltf => {
      const model = gltf.scene;
      if (this.disposed) {
        this.clear(model);
        return;
      }
      model.rotation.x = Math.PI / 2; // glTF's +y (up) becomes the page's +z
      // The surfaces with a drawn look become the stage's own; the frame keeps the file's steel
      model.traverse(object => {
        if (!(object instanceof Mesh) || Array.isArray(object.material)) return;
        const material = object.material as Material;
        const own = this.deskMaterials.get(material.name);
        if (!own) return;
        material.dispose();
        object.material = own;
      });
      this.model = model;
      this.table.add(model);
      this.showTop();
      this.toBackdropLayer();
    }, undefined, error => {
      console.warn("[mock-stage] the desk could not be loaded:", error);
    });
  }

  /** Shows the chosen top's mesh and hides the other's (nothing before the model is loaded) */
  private showTop(): void {
    if (!this.model) return;
    for (const [id, name] of Object.entries(TOP_NODES)) {
      const node = this.model.getObjectByName(name);
      if (node) node.visible = id === this.topId;
    }
  }

  /**
   * A top's look, its textures loaded or drawn on first use: the screen-printed plywood with the plies on its edges, or
   * the linoleum with the same plies in oak on its edges and the oak veneer underneath
   */
  private applyTopLook(id: TopId): void {
    if (this.looks.has(id)) return;
    this.looks.add(id);
    this.plies ??= plywoodEdgeTexture({ plies: DESK_EDGE_PLIES });
    if (id === "linoleum") {
      this.linoleum ??= linoleumTexture();
      this.oak ??= oakTexture();
      this.linoleumMaterial.map = this.linoleum;
      this.oakMaterial.map = this.oak;
      this.oakEdgeMaterial.map = this.plies;
      [this.linoleumMaterial, this.oakMaterial, this.oakEdgeMaterial].forEach(material => (material.needsUpdate = true));
      return;
    }
    const top = this.topMaterial;
    const edge = this.edgeMaterial;
    const desk = this.ensureDesk();
    top.map = desk.map;
    top.roughnessMap = desk.roughnessMap;
    top.color.setHex(DESK_TINT);
    top.roughness = 0.5;
    edge.map = this.plies;
    edge.roughnessMap = null;
    edge.color.setHex(DESK_EDGE_TINT);
    edge.roughness = DESK_EDGE_ROUGHNESS;
    top.needsUpdate = true;
    edge.needsUpdate = true;
  }

  /**
   * The open scene on the stage (#319: plain boards): each surface a board at its place – its face paper, a wall stands
   * – with its reference image (`scene.references`) fitted onto it; the targets' prints on their surfaces. No book, no
   * fold, no cover: a scene is its surfaces.
   */
  setLayout(targets: readonly StageTarget[], scene: StageScene): void {
    const run = ++this.layoutRun;
    this.clear(this.boards);
    this.clear(this.pages);
    this.textures.forEach(texture => texture.dispose());
    this.textures = [];
    this.labelTextures.forEach(texture => texture.dispose());
    this.labelTextures.clear();

    this.frames = scene.surfaces.length ? scene.surfaces.map(surfaceFrame) : [DEFAULT_FRAME];
    const edge = new MeshStandardMaterial({ color: PAPER_EDGE_COLOR, roughness: 1 });
    const face = new MeshStandardMaterial({ color: PAPER_COLOR, roughness: 1 });
    this.frames.forEach(frame => {
      const board = new Mesh(new BoxGeometry(frame.widthMm / 1000, frame.heightMm / 1000, BOARD_THICKNESS), [edge, edge, edge, edge, face, edge]);
      layOnSurface(board, frame, frame.widthMm / 2, frame.heightMm / 2, FACE - BOARD_THICKNESS / 2);
      board.userData.surfaceId = frame.id;
      this.boards.add(board);
    });
    this.layReferences(scene.references ?? {}, run);

    targets.forEach(target => {
      // A unit plane scaled to the print: a move or the image's own aspect only rescales it, so whatever hangs on it
      // (the selection's outline) follows
      const plane = new Mesh(new PlaneGeometry(1, 1), new MeshBasicMaterial({ color: 0xffffff }));
      plane.userData.targetId = target.id;
      this.placePrint(plane, target);
      this.pages.add(plane);
      this.loader.setCrossOrigin("anonymous");
      this.loader.load(target.imageSrc, texture => {
        if (run !== this.layoutRun) {
          texture.dispose();
          return;
        }
        texture.colorSpace = SRGBColorSpace;
        this.textures.push(texture);
        // The printed width is known; the height follows the image, as the print does
        this.aspects.set(target.imageSrc, texture.image.height / texture.image.width);
        this.placePrint(plane, plane.userData.place as StageTarget);
        const own = ownMaterial<MeshBasicMaterial>(plane);
        own.map = texture;
        own.needsUpdate = true;
      });
    });
    this.drawGrid();
    this.drawMarks();
    this.drawTargetMarks();
    this.toBackdropLayer();
  }

  /** The target marks on or off (#323): every target's place framed with its id */
  setTargetMarks(on: boolean): void {
    if (this.targetMarks === on) return;
    this.targetMarks = on;
    this.drawTargetMarks();
    this.toBackdropLayer();
  }

  /** The marks follow a print that moved or learned its height (its image loaded) – once for all that changed together */
  private queueTargetMarks(): void {
    if (!this.targetMarks || this.targetMarksQueued) return;
    this.targetMarksQueued = true;
    queueMicrotask(() => {
      this.targetMarksQueued = false;
      if (this.disposed) return;
      this.drawTargetMarks();
      this.toBackdropLayer();
    });
  }

  /**
   * Frames each target's print where it lies – the print's own box, its height from the image once loaded – green and
   * solid, or orange and dashed when switched off, through everything like the surfaces' outlines; its id on a label
   * just above the frame's top left
   */
  private drawTargetMarks(): void {
    this.targetMarksGroup.children.forEach(child => {
      if (child instanceof Mesh) {
        child.geometry.dispose();
        (child.material as Material).dispose();
      }
    });
    clearLines(this.targetMarksGroup);
    if (!this.targetMarks) return;
    const solid: number[] = [];
    const dashed: number[] = [];
    const at = (frame: SurfaceFrame, x: number, y: number) => toStage(frame, x, y, 0).map(v => v / 1000);
    for (const plane of this.targetPlanes()) {
      const target = plane.userData.place as StageTarget | undefined;
      if (!target) continue;
      const frame = frameFor(this.frames, target.surface);
      const height = placeHeight(target, this.aspects.get(target.imageSrc));
      const { x, y, width } = target;
      const corners = [[x, y], [x + width, y], [x + width, y + height], [x, y + height]];
      corners.forEach(([ax, ay], i) => {
        const [bx, by] = corners[(i + 1) % 4];
        const pieces = target.off ? dashes(ax, ay, bx, by) : [[ax, ay, bx, by]];
        pieces.forEach(([x1, y1, x2, y2]) => (target.off ? dashed : solid).push(...at(frame, x1, y1), ...at(frame, x2, y2)));
      });
      const label = this.markLabel(target, frame);
      if (label) this.targetMarksGroup.add(label);
    }
    addLines(this.targetMarksGroup, solid, TARGET_ON_COLOR, 1, true);
    addLines(this.targetMarksGroup, dashed, TARGET_OFF_COLOR, 1, true);
  }

  /** A target mark's label: its id, LABEL_MM high, its bottom left on the frame's top left corner */
  private markLabel(target: StageTarget, frame: SurfaceFrame): Mesh | null {
    const key = `${target.id}|${target.off ? "off" : "on"}`;
    let texture = this.labelTextures.get(key) ?? null;
    if (!texture) {
      texture = labelTexture(target.id, target.off ? TARGET_OFF_CSS : TARGET_ON_CSS);
      if (!texture) return null;
      this.labelTextures.set(key, texture);
    }
    const image = texture.image as { width: number; height: number };
    const width = (LABEL_MM * image.width) / image.height;
    const label = new Mesh(new PlaneGeometry(width / 1000, LABEL_MM / 1000), new MeshBasicMaterial({ map: texture, transparent: true, depthTest: false, depthWrite: false }));
    label.renderOrder = 2;
    label.userData.targetMark = target.id;
    layOnSurface(label, frame, target.x + width / 2, target.y - LABEL_MM / 2, 0);
    return label;
  }

  /**
   * The surfaces' reference images (#314): on the board's face, under the prints, fitted to the surface as its `fit`
   * says (`fitImage`) – paper around a contained one
   */
  private layReferences(references: Readonly<Record<string, SurfaceReference>>, run: number): void {
    for (const frame of this.frames) {
      const reference = references[frame.id];
      if (!reference) continue;
      const plane = new Mesh(new PlaneGeometry(1, 1), new MeshBasicMaterial({ color: PAPER_COLOR }));
      plane.scale.set(frame.widthMm / 1000, frame.heightMm / 1000, 1);
      layOnSurface(plane, frame, frame.widthMm / 2, frame.heightMm / 2, -2 * LAYER);
      plane.userData.reference = frame.id;
      this.boards.add(plane);
      this.loader.setCrossOrigin("anonymous");
      this.loader.load(reference.url, texture => {
        if (run !== this.layoutRun) {
          texture.dispose();
          return;
        }
        texture.colorSpace = SRGBColorSpace;
        texture.anisotropy = 8;
        const fitted = fitImage(reference.fit, texture.image.height / texture.image.width, frame.widthMm, frame.heightMm);
        texture.repeat.set(...fitted.repeat);
        texture.offset.set(...fitted.offset);
        this.textures.push(texture);
        plane.scale.set(fitted.width / 1000, fitted.height / 1000, 1);
        const own = ownMaterial<MeshBasicMaterial>(plane);
        own.map = texture;
        own.color.setHex(0xffffff);
        own.needsUpdate = true;
      }, undefined, error => console.warn("[mock-stage] a reference image could not be loaded:", reference.url, error));
    }
  }

  /** Outline the surfaces (the Surface mode, #314): `outlined` thin, `highlighted` in the highlight's colour; unknown ids are left out */
  setSurfaceMarks(outlined: readonly string[], highlighted: string | null): void {
    this.marks = { outlined: [...outlined], highlighted };
    this.drawMarks();
    this.toBackdropLayer();
  }

  /** The surfaces the stage knows now, in the stage frame (mm) */
  surfaceFrames(): readonly SurfaceFrame[] {
    return this.frames;
  }

  /** The surface under the pointer in the Surface mode (#314): outlined stronger than the others, under the highlight */
  setSurfaceHover(id: string | null): void {
    if (id === this.hoveredSurface) return;
    this.hoveredSurface = id;
    this.drawMarks();
    this.toBackdropLayer();
  }
  private hoveredSurface: string | null = null;

  /** An editor helper that lives in the stage frame (moves with the stage): the surface gizmo's stand-in */
  addHelper(object: Object3D): void {
    this.group.add(object);
  }

  private drawMarks(): void {
    clearLines(this.marksGroup);
    // A loop `inset` mm inside the surface's edges
    const outline = (frame: SurfaceFrame, inset = 0): number[] => {
      const a = Math.min(inset, frame.widthMm / 4, frame.heightMm / 4);
      const corners = [[a, a], [frame.widthMm - a, a], [frame.widthMm - a, frame.heightMm - a], [a, frame.heightMm - a]].map(([x, y]) => toStage(frame, x, y, 0));
      return corners.flatMap((corner, i) => [...corner, ...corners[(i + 1) % 4]].map(v => v / 1000));
    };
    const { highlighted } = this.marks;
    const hovered = this.hoveredSurface !== highlighted ? this.hoveredSurface : null;
    const outlined = this.frames.filter(f => this.marks.outlined.includes(f.id) && f.id !== highlighted && f.id !== hovered);
    addLines(this.marksGroup, outlined.flatMap(f => outline(f)), SURFACE_OUTLINE_COLOR, 0.9, true);
    const hover = this.frames.find(f => f.id === hovered);
    if (hover) addLines(this.marksGroup, [...outline(hover), ...outline(hover, 1)], SURFACE_HOVER_COLOR, 1, true);
    // The selected one in two tones (monochrome, RULES #38): white on the dark desk, black just inside on the paper
    const selected = this.frames.find(f => f.id === highlighted);
    if (selected) {
      addLines(this.marksGroup, [...outline(selected), ...outline(selected, 0.6)], SURFACE_HIGHLIGHT_COLOR, 1, true);
      addLines(this.marksGroup, [...outline(selected, 1.4), ...outline(selected, 2)], SURFACE_HIGHLIGHT_INNER_COLOR, 1, true);
    }
  }
  /** The targets hidden in the tree (the eye): their printed images go with their AR content, so the scene shows neither */
  setHiddenPrints(hidden: ReadonlySet<string>): void {
    this.targetPlanes().forEach(plane => {
      plane.visible = !hidden.has(plane.userData.targetId as string);
    });
  }

  /** Lay a print's plane where its target lies: one layer under the target's own plane (the entities sit at its z = 0) */
  private placePrint(plane: Mesh, target: StageTarget): void {
    plane.userData.place = target;
    const aspect = this.aspects.get(target.imageSrc);
    const height = placeHeight(target, aspect);
    plane.scale.set(target.width / 1000, height / 1000, 1);
    // On its surface (#314): flat on the table as before, standing on a wall
    layOnSurface(plane, frameFor(this.frames, target.surface), target.x + target.width / 2, target.y + height / 2, -LAYER);
    this.queueTargetMarks();
  }

  /** A target's print moves (the placement being edited): the same plane, so its texture and outline stay */
  movePrint(target: StageTarget): void {
    const plane = this.targetPlanes().find(p => p.userData.targetId === target.id);
    if (plane) this.placePrint(plane, target);
  }

  /** A printed image's height per width, once its image has loaded */
  printedAspect(targetId: string): number | undefined {
    const place = this.targetPlanes().find(p => p.userData.targetId === targetId)?.userData.place as StageTarget | undefined;
    return place ? this.aspects.get(place.imageSrc) : undefined;
  }

  /** The stage's frame (metres, origin at the scene's origin, z up) – snapping counts in it */
  frame(): Object3D {
    this.group.updateMatrixWorld(true);
    return this.group;
  }

  /** The grids (#316, #319): the surfaces' own, the floor grid, the vertical grid; each off = none */
  setGrid(grid: StageGrids): void {
    this.grid = grid;
    this.drawGrid();
    this.toBackdropLayer();
  }

  /** The frame the guides and the flat surfaces' snapping count on: the scene's surfaces lying flat on the table */
  private gridFrame(): OriginFrame {
    return { surfaces: surfaceRects(this.frames) };
  }

  /**
   * Each surface's **own grid** (#316 part 1) for the targets on it: lines every `mm` (the surface's own spacing, else
   * the grid's) from the origin's zero – a corner or an edge's centre of that surface, or the scene's origin (`centre`)
   * on a flat one, a wall's middle on a standing one – the zero lines in another colour, just above the prints and below
   * the entities. The vertical axis rises from each zero, out of the surface. Then the scene's grids (`drawSceneGrid`).
   */
  private drawGrid(): void {
    clearLines(this.gridGroup);
    clearLines(this.sceneGridGroup);
    this.drawSceneGrid();
    const lines: number[] = [];
    const fine: number[] = [];
    const zeros: number[] = [];
    const verticals: number[] = [];
    const frame = this.gridFrame();
    const lift = (LAYER / 2) * 1000;
    const push = (target: number[], surface: SurfaceFrame, x: number, y: number) => target.push(...toStage(surface, x, y, -lift).map(v => v / 1000));
    for (const surface of this.frames) {
      // The surface's own grid (#316, #314): its switch, spacing and subdivisions, else the toolbar's
      const own = this.grid.surfaces[surface.id] ?? {};
      if (!(own.on ?? this.grid.on)) continue;
      const step = own.mm ?? this.grid.mm;
      const zero = surface.flat || this.grid.origin !== "centre"
        ? (surface.flat ? originPoint(this.grid.origin, frame, surface.id) : spotOf(this.grid.origin, surface))
        : { x: surface.widthMm / 2, y: surface.heightMm / 2 };
      for (const x of gridLines(zero.x, 0, surface.widthMm, step)) {
        const target = Math.abs(x - zero.x) < 1e-6 ? zeros : lines;
        push(target, surface, x, 0);
        push(target, surface, x, surface.heightMm);
      }
      for (const y of gridLines(zero.y, 0, surface.heightMm, step)) {
        const target = Math.abs(y - zero.y) < 1e-6 ? zeros : lines;
        push(target, surface, 0, y);
        push(target, surface, surface.widthMm, y);
      }
      for (const x of subdivisionLines(zero.x, 0, surface.widthMm, step, own.sub ?? 1)) {
        push(fine, surface, x, 0);
        push(fine, surface, x, surface.heightMm);
      }
      for (const y of subdivisionLines(zero.y, 0, surface.heightMm, step, own.sub ?? 1)) {
        push(fine, surface, 0, y);
        push(fine, surface, surface.widthMm, y);
      }
      if (this.grid.axis && zero.x >= 0 && zero.x <= surface.widthMm && zero.y >= 0 && zero.y <= surface.heightMm) {
        verticals.push(...toStage(surface, zero.x, zero.y, -lift).map(v => v / 1000), ...toStage(surface, zero.x, zero.y, GRID_AXIS_MM).map(v => v / 1000));
      }
    }
    addLines(this.gridGroup, fine, GRID_COLOR, 0.18);
    addLines(this.gridGroup, lines, GRID_COLOR, 0.45);
    addLines(this.gridGroup, zeros, GRID_ZERO_COLOR, 0.9);
    addLines(this.gridGroup, verticals, GRID_AXIS_COLOR, 0.9);
  }

  /**
   * The scene's grids (#319): the **floor grid** – a transparent net on the table (the scene's y = 0), every `floorMm`
   * through the origin, every tenth line stronger – and, switched on, the **vertical grid**: the same net standing at the
   * scene's z = `wallZ` (stage y = −wallZ), from the floor up. With the axis: the scene's axes at its origin.
   */
  private drawSceneGrid(): void {
    const { floor, wall, floorMm, wallZ } = this.grid;
    // Never more lines than the stage can draw lightly: a fine spacing over the whole extent steps up
    const step = Math.max(floorMm, (2 * FLOOR_GRID_EXTENT_MM) / FLOOR_GRID_MAX_LINES);
    const extent = FLOOR_GRID_EXTENT_MM;
    const major = (at: number) => Math.abs(Math.round(at / step) % 10) === 0;
    if (floor) {
      const z = this.tableTop + LAYER / 4;
      const minor: number[] = [];
      const strong: number[] = [];
      for (const x of gridLines(0, -extent, 2 * extent, step)) (major(x) ? strong : minor).push(x / 1000, -extent / 1000, z, x / 1000, extent / 1000, z);
      for (const y of gridLines(0, -extent, 2 * extent, step)) (major(y) ? strong : minor).push(-extent / 1000, y / 1000, z, extent / 1000, y / 1000, z);
      addLines(this.sceneGridGroup, minor, FLOOR_GRID_COLOR, 0.22);
      addLines(this.sceneGridGroup, strong, FLOOR_GRID_COLOR, 0.45);
    }
    if (wall) {
      const y = -wallZ / 1000;
      const minor: number[] = [];
      const strong: number[] = [];
      for (const x of gridLines(0, -extent, 2 * extent, step)) (major(x) ? strong : minor).push(x / 1000, y, this.tableTop, x / 1000, y, WALL_GRID_HEIGHT_MM / 1000);
      for (const h of gridLines(0, 0, WALL_GRID_HEIGHT_MM, step)) (major(h) ? strong : minor).push(-extent / 1000, y, h / 1000, extent / 1000, y, h / 1000);
      addLines(this.sceneGridGroup, minor, FLOOR_GRID_COLOR, 0.28);
      addLines(this.sceneGridGroup, strong, FLOOR_GRID_COLOR, 0.55);
    }
    // The origin gizmo (#316, #314): with the axis switch, whatever grid is on – the scene's axes with arrowheads and a
    // ring round the origin, drawn through everything (the stage frame: scene x = stage x, y up = stage z, z = stage −y)
    if (!this.grid.axis) return;
    const gizmo = originGizmo(SCENE_AXIS_MM);
    addLines(this.sceneGridGroup, gizmo.x, AXIS_X_COLOR, 1, true);
    addLines(this.sceneGridGroup, gizmo.y, GRID_AXIS_COLOR, 1, true);
    addLines(this.sceneGridGroup, gizmo.z, AXIS_Z_COLOR, 1, true);
    addLines(this.sceneGridGroup, gizmo.ring, ORIGIN_RING_COLOR, 0.9, true);
  }

  /**
   * The snap guides: a line across the row for each line a drag snapped to (x = a line up the page, y = one across
   * it, in mm of the stage frame); an empty list clears them. They lie on the page and show through the entities.
   */
  setGuides(guides: ReadonlyArray<{ axis: 0 | 1 | 2; at: number }>): void {
    clearLines(this.guideGroup);
    const box = frameBox(this.gridFrame());
    const [left, right] = [box.left - GUIDE_OVERHANG_MM, box.left + box.width + GUIDE_OVERHANG_MM];
    const [high, low] = [box.top + GUIDE_OVERHANG_MM, box.top - box.height - GUIDE_OVERHANG_MM];
    const positions: number[] = [];
    for (const { axis, at } of guides) {
      if (axis === 0) positions.push(at / 1000, low / 1000, 0, at / 1000, high / 1000, 0);
      if (axis === 1) positions.push(left / 1000, at / 1000, 0, right / 1000, at / 1000, 0);
    }
    addLines(this.guideGroup, positions, GUIDE_COLOR, 1, true);
    this.toBackdropLayer();
  }

  /**
   * Everything of the stage stands in for the camera picture: on the engine's backdrop layer the scene's
   * post-processing passes shade it as the camera picture (bloom, vignette; the grade stays on the AR layer)
   */
  private toBackdropLayer(): void {
    this.group.traverse(object => object.layers.set(BACKDROP_LAYER));
    // Called whenever the stage built something new: it gets the current shading as well
    this.shadeStage();
  }

  /**
   * The sky and the light: the HDRI while rendered, else the scene's own lights over the editor's dark ground – or the
   * environment's world (drawn by the engine under the stage) with the scene's own lights (`sky.ts`)
   */
  setHdri(hdri: HdriId | null): void {
    this.hdri = hdri;
    this.wantedHdri = this.look().hdri;
    if (!this.wantedHdri) {
      this.scene.background = null;
      this.scene.environment = null;
      if (this.sky) this.sky.visible = false;
      this.dimLights(false);
      return;
    }
    const loaded = this.hdris.get(this.wantedHdri);
    if (loaded) {
      this.applyHdri(loaded);
      return;
    }
    const id = this.wantedHdri;
    new RGBELoader().load(this.assetUrl(`hdri/${id}_1k.hdr`), texture => {
      texture.mapping = EquirectangularReflectionMapping;
      this.hdris.set(id, texture);
      if (this.wantedHdri === id) this.applyHdri(texture);
    }, undefined, error => console.warn(`[mock-stage] HDRI "${id}" could not be loaded:`, error));
  }

  /** The ghost's parts for the handles: the image and its corner (none while no ghost is shown) */
  ghostParts(): { image: Object3D | null; corner: Object3D | null } {
    return {
      image: this.ghostGroup.getObjectByName("ghost") ?? null,
      corner: this.ghostGroup.getObjectByName("ghost-corner") ?? null,
    };
  }

  /**
   * Where a world ray meets the scene's surfaces (#314, `hitSurfaces`): the surface it hits (by id) and mm from its top
   * left corner – the nearest surface hit inside its edges, else the one whose edge is nearest (a drag beyond a page's
   * edge stays on it; on the open book the spine decides, as before)
   */
  surfaceHit(raycaster: Raycaster): { surface: StageSurface; x: number; y: number } | null {
    this.group.updateMatrixWorld(true);
    const inverse = this.group.matrixWorld.clone().invert();
    const origin = raycaster.ray.origin.clone().applyMatrix4(inverse).multiplyScalar(1000);
    const direction = raycaster.ray.direction.clone().transformDirection(inverse);
    return hitSurfaces(this.frames, [origin.x, origin.y, origin.z], [direction.x, direction.y, direction.z]);
  }

  /** The printed images on the page (each knows its `userData.targetId`) – for picking a target by its print */
  targetPlanes(): Mesh[] {
    return this.pages.children.filter((child): child is Mesh => child instanceof Mesh);
  }

  /**
   * A translucent print of `ghost` where it would lie (above the print, below the entities), with a small square at
   * its bottom right corner – the handles: drag the image to move it, the corner to set its printed width. With
   * `handlesOnly` the target's own print moves instead (`movePrint`): the image stays a handle but is not drawn, so
   * the target never shows twice.
   */
  setGhost(ghost: StageTarget | null, options: { handlesOnly?: boolean } = {}): void {
    this.clear(this.ghostGroup);
    if (!ghost) return;
    const material = new MeshBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.75, depthWrite: false });
    material.visible = !options.handlesOnly;
    const plane = new Mesh(new PlaneGeometry(1, 1), material);
    plane.name = "ghost";
    const corner = new Mesh(new PlaneGeometry(GHOST_HANDLE / 1000, GHOST_HANDLE / 1000), new MeshBasicMaterial({ color: 0x1e1e1e, transparent: true, opacity: 0.9, depthWrite: false }));
    corner.name = "ghost-corner";
    const place = () => {
      const aspect = this.aspects.get(ghost.imageSrc);
      const height = placeHeight(ghost, aspect);
      const surface = frameFor(this.frames, ghost.surface);
      plane.scale.set(ghost.width / 1000, height / 1000, 1);
      layOnSurface(plane, surface, ghost.x + ghost.width / 2, ghost.y + height / 2, -LAYER / 2);
      layOnSurface(corner, surface, ghost.x + ghost.width, ghost.y + height, 0);
    };
    place();
    this.ghostGroup.add(plane, corner);
    this.toBackdropLayer();
    const apply = (texture: Texture) => {
      if (!plane.parent) return;
      this.aspects.set(ghost.imageSrc, texture.image.height / texture.image.width);
      place();
      material.map = texture;
      material.needsUpdate = true;
    };
    const cached = this.ghostTextures.get(ghost.imageSrc);
    if (cached) {
      apply(cached);
      return;
    }
    this.loader.setCrossOrigin("anonymous");
    this.loader.load(ghost.imageSrc, texture => {
      texture.colorSpace = SRGBColorSpace;
      this.ghostTextures.set(ghost.imageSrc, texture);
      apply(texture);
    });
  }

  private applyHdri(texture: Texture): void {
    const exposure = HDRI_EXPOSURE[this.wantedHdri ?? ""] ?? HDRI_EXPOSURE_DEFAULT;
    const look = this.look();
    // The light: the equirectangular map as the scene's environment (its direction does not follow the orbit – the
    // room's light is even enough for that not to show) – not while the world is the sky (the reader's lights, `sky.ts`)
    this.scene.background = null;
    this.scene.environment = look.light === "hdri" ? texture : null;
    this.scene.environmentIntensity = exposure;
    // The sky: the same image on a sphere around the stage, seen from inside; the HDRI's up is the page's +z
    const map = texture.clone();
    map.mapping = UVMapping;
    map.needsUpdate = true;
    if (!this.sky) {
      const geometry = new SphereGeometry(SKY_RADIUS, 64, 32);
      geometry.scale(-1, 1, 1); // seen from inside, the image reads the right way round
      this.sky = new Mesh(geometry, new MeshBasicMaterial({ toneMapped: false, depthWrite: false }));
      this.sky.name = "mock-sky";
      this.sky.rotation.x = Math.PI / 2;
      this.sky.renderOrder = -1;
      this.group.add(this.sky);
      this.toBackdropLayer();
    }
    this.sky.material.map?.dispose();
    this.sky.material.map = map;
    this.sky.material.color.setScalar(exposure);
    behindWorld(this.sky.material, look.behindWorld);
    this.sky.material.needsUpdate = true;
    this.sky.visible = look.sky === "hdri" || look.behindWorld;
    this.dimLights(look.light === "hdri");
  }

  /**
   * The scene's own lights (the engine's ambient and sun, children of the scene) step back while the HDRI lights the
   * stage; the content's lights (light nodes, inside the anchors) keep the author's intensity
   */
  private dimLights(on: boolean): void {
    if (on) {
      this.scene.children.forEach(object => {
        if (object instanceof Light && !this.dimmed.has(object)) {
          this.dimmed.set(object, object.intensity);
          object.intensity *= LIGHTS_WITH_HDRI;
        }
      });
    } else {
      this.dimmed.forEach((intensity, light) => (light.intensity = intensity));
      this.dimmed.clear();
    }
  }

  private ensureDesk(): DeskTextures {
    if (this.desk) return this.desk;
    const load = (path: string, srgb: boolean, repeat: readonly [number, number]) => {
      const texture = this.loader.load(this.assetUrl(path));
      texture.wrapS = texture.wrapT = RepeatWrapping;
      texture.repeat.set(repeat[0], repeat[1]);
      if (srgb) texture.colorSpace = SRGBColorSpace;
      return texture;
    };
    this.desk = {
      map: load(DESK_DIFFUSE, true, DESK_REPEAT),
      roughnessMap: load(DESK_ROUGHNESS, false, DESK_REPEAT),
    };
    return this.desk;
  }

  update(cameraInverse: Matrix4): void {
    this.group.matrix.copy(cameraInverse);
    this.group.matrixWorldNeedsUpdate = true;
  }

  dispose(): void {
    this.disposed = true;
    this.layoutRun++;
    this.clear(this.boards);
    this.clear(this.pages);
    this.clear(this.ghostGroup);
    clearLines(this.gridGroup);
    clearLines(this.sceneGridGroup);
    clearLines(this.guideGroup);
    clearLines(this.marksGroup);
    this.targetMarks = false;
    this.drawTargetMarks();
    this.labelTextures.forEach(texture => texture.dispose());
    this.labelTextures.clear();
    this.ghostTextures.forEach(texture => texture.dispose());
    this.ghostTextures.clear();
    this.clear(this.table);
    this.model = null;
    this.deskMaterials.forEach(material => material.dispose());
    this.textures.forEach(texture => texture.dispose());
    this.textures = [];
    if (this.desk) Object.values(this.desk).forEach(texture => texture.dispose());
    this.desk = null;
    [this.plies, this.linoleum, this.oak].forEach(texture => texture?.dispose());
    this.plies = this.linoleum = this.oak = null;
    this.hdris.forEach(texture => texture.dispose());
    this.hdris.clear();
    if (this.sky) {
      this.sky.geometry.dispose();
      this.sky.material.map?.dispose();
      this.sky.material.dispose();
      this.sky.removeFromParent();
      this.sky = null;
    }
    this.scene.background = null;
    this.scene.environment = null;
    this.dimLights(false);
    this.group.removeFromParent();
  }

  /** Empties `group`, freeing every mesh below it (the desk's shared materials are the stage's own, disposed apart) */
  private clear(group: Object3D): void {
    [...group.children].forEach(child => {
      child.traverse(object => {
        if (!(object instanceof Mesh)) return;
        object.geometry.dispose();
        (Array.isArray(object.material) ? object.material : [object.material]).forEach(material => {
          if (![...this.deskMaterials.values()].includes(material as MeshStandardMaterial)) material.dispose();
        });
      });
      group.remove(child);
    });
  }
}
