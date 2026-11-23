import type { BehaviourAction } from "@engine/types";

/**
 * The **logic model** shared by every level of the Logic view (#334, `agents/research/2026-11-20-logic-nodes-shared-concept.md`
 * §5.2): the documents are the logic, the graph is a view of them. A `View` is computed from the draft for one level
 * (`Context`) and never stored; each edge that a document holds names that document's place (`Home`), so an edit can later
 * become an op on that one document (L-T4). Edges that only join by name – the bus (`emit doorOpened` → `on doorOpened`), a
 * variable set in one scene and read in another, a plugin's code – have no home (`null`): drawing them changes nothing.
 * Pure types: no DOM, no store – the folder moves into a package with #255.
 */

/**
 * What flows along an edge and through a port (a subset of SCUTOONS' deck-kit `Signal`, same names); `media` (#334 L-T6) is a
 * file – a picture, a sound, a video, a model (SCUTOONS' `Picture` / `Audio`)
 */
export type Signal = "event" | "value" | "node" | "scene" | "address" | "far" | "media";

/**
 * The type of a port that comes from a spec (B8, research §5.9.3): a parameter's kind with its hard limits or options, a
 * file's kind. Connecting is strictly typed: a port takes only its own type, an integer also where a number goes.
 */
export type PortType =
  | { kind: "number"; min: number; max: number }
  | { kind: "integer"; min: number; max: number }
  | { kind: "boolean" }
  | { kind: "string" }
  | { kind: "color" }
  | { kind: "choice"; options: readonly string[] }
  | { kind: "texture" }
  | { kind: "model" }
  | { kind: "file"; accepts: string };

/** Where a spec's port comes from: a parameter, the node's file, a verb it answers, an event it emits */
export type PortKind = "param" | "src" | "action" | "event";

export interface PortSpec {
  /** Unique on its node: `on`, `when`, `do:0`, `out`; a spec's: `p:volume`, `p:hum.volume`, `src`, `act:play`, `ev:ended` */
  id: string;
  label: string;
  signal: Signal;
  dir: "in" | "out";
  /** It takes or gives more than one edge */
  many?: boolean;
  /** A port derived from a spec (§5.9): what it stands for */
  kind?: PortKind;
  /** A value or file port's type */
  type?: PortType;
  /** Whose it is: the node type's own (`video`) or a component's type (`hum`) */
  owner?: string;
  /** Hidden on the box until it is wired or exposed (the Blender way – unused ports hide) */
  hidden?: boolean;
}

/** The families of node types; `free` is room for canvas nodes that no document projects (§5.7, none yet) */
export type NodeFamily = "event" | "rule" | "action" | "variable" | "entity" | "scene" | "plugin" | "far" | "free";

/** Who registered a node type: the core, a title (an app built on the core) or a plugin by its id */
export type NodeOwner = "core" | (string & {});

/** A node type of the registry (`registry.ts`): its family, its ports for the params of one node, its owner */
export interface NodeType<P = unknown> {
  kind: string;
  family: NodeFamily;
  ports(params: P): PortSpec[];
  owner: NodeOwner;
}

/** Where the Logic view stands: the whole experience, a group, a scene, or one target's entity tree (`path`: its subtree) */
export type Context =
  | { level: "experience" }
  | { level: "group"; group: string }
  | { level: "scene"; scene: string }
  | { level: "entity"; scene: string; target: string; path: number[] };

export type Level = Context["level"];

/** The part of a rule an edge stands for; `index`: the variable of `when` (in the order the expression reads them), the action of `do` */
type RulePart = { part: "on" | "when" | "do"; index?: number };

/**
 * A rule of the documents. `entity`: a rule of a `behaviour` component of a target's node (`component` is the node's
 * component index, `rule` the rule's index in it); `scene`: a rule of a scene's `logic`; `experience`: a rule of the
 * experience's `logic` (content schema 5.0)
 */
export type RuleRef =
  | { doc: "entity"; target: string; path: number[]; component: number; rule: number }
  | { doc: "scene"; scene: string; rule: number }
  | { doc: "experience"; rule: number };

/** The document place an edge comes from: a rule and the part of it */
export type Home = RuleRef & RulePart;

/** What a click on a box selects in the workspace */
export type LogicPick =
  | { kind: "node"; target: string; path: number[] }
  | { kind: "plugin"; plugin: string }
  | { kind: "scene"; scene: string }
  | { kind: "group"; group: string }
  /** The experience's rules: the experience in the Inspector */
  | { kind: "experience" };

export interface GraphNode {
  id: string;
  /** Its node type's kind (`registry.ts`) */
  kind: string;
  /** The line it shows */
  label: string;
  /** The smaller second line: whose event, whose rule, the scopes a variable is set in, what a plugin is */
  detail: string;
  ports: PortSpec[];
  pick: LogicPick | null;
  /** The level a double-click opens (a group, a scene, an entity) */
  enter?: Context;
  /** What is wrong with it, if anything (an error drawn in the error colour, a warning dashed) */
  problem?: string;
  /** How bad its problem is: an error (a broken name, a refused type) or a warning (nothing listens to, sets or emits it) */
  problemSeverity?: Severity;
  /** It stands for something outside this level */
  far?: boolean;
  /** The id of the group node it sits in (a frame) */
  group?: string;
  /** A rule box: where its rule is written (part \`on\`) – a rule whose on was cut has no wire that says so */
  home?: Home;
  /** A picture its body shows (an image node's file) – cheap previews only; the choice of preview is L-T15 */
  preview?: string;
  /**
   * Display rows (#334, Tilman 2026-11-21): what a plugin's code uses that no wire can change – what it reads, sets, drives –
   * as plain text under the pins, each row's names joined to their boxes by a dotted reference line (`View.refs`)
   */
  rows?: DisplayRow[];
  /** A plugin box: the events its `emits` outlet gives and its `listens` inlet takes (its declared uses) – a wire picks one */
  bus?: { emits: string[]; listens: string[] };
}

/** A display row of a box: the verb (`reads`, `sets`, `drives`) and the names – the text `reads hasKey, scene` */
export interface DisplayRow {
  id: "reads" | "sets" | "drives" | "goto";
  names: string[];
}

/**
 * A reference from a box's display row to the box it names (#334): drawn as a fine dotted line, never a wire – not
 * selectable, not deletable, no op names it. `dir` says which way the use runs for the layout: `in` – the row reads from the
 * box (a variable read), `out` – the row reaches the box (a variable set, a node driven).
 */
export interface Reference {
  id: string;
  from: [node: string, row: DisplayRow["id"]];
  to: string;
  dir: "in" | "out";
}

export interface GraphEdge {
  id: string;
  from: [node: string, port: string];
  to: [node: string, port: string];
  signal: Signal;
  /** The word on the edge: `on`, `when`, `set = true`, `emit`, `show`, a plugin's type, `event · 2` … */
  label: string;
  /** The document place it comes from; null = derived (a by-name join: the bus, a variable across scenes, a plugin's use) */
  home: Home | null;
  /**
   * The events or variables a derived edge joins by name, when its ends do not say it (a bundled wire between two scenes,
   * a stub at the border) – the live feedback lights it when one of them fires or changes (`live.ts`, #334)
   */
  names?: readonly string[];
}

/** An edge leaving the level: drawn at its border, named by its far end; following it opens `far` */
export interface Stub {
  edge: GraphEdge;
  /** `in`: it comes from outside to a box here; `out`: from a box here to outside */
  side: "in" | "out";
  far: Context;
  label: string;
}

/** An error: something broken (a name that is not there, a type the rule refuses); a warning: it never does anything */
export type Severity = "error" | "warning";

export interface Problem {
  message: string;
  severity: Severity;
  /** The box it is about, when it is one of this view's */
  node?: string;
  /** The box's name (or the rule's place) – the issues bar shows it, so `short` need not say it */
  place?: string;
  /** The message without what `place` already says */
  short?: string;
  /** Where the box itself is drawn when `node` is only its container here (a scene's box at the experience level) */
  at?: { context: Context; node: string };
}

/** A free node on the canvas (§5.7) – declared only; the canvas comes with L-T7 */
export interface CanvasNode {
  id: string;
  kind: string;
  label: string;
  params: Record<string, unknown>;
}

/** The stored canvas beside a rules list (decision L4): groups, notes, folds, free nodes – empty until L-T7 */
export interface Canvas {
  nodes: CanvasNode[];
  wires: Array<{ from: [string, string]; to: [string, string] }>;
}

export interface View {
  context: Context;
  /** The levels from the experience down to this one, root first (the breadcrumb) */
  levels: Context[];
  nodes: GraphNode[];
  edges: GraphEdge[];
  stubs: Stub[];
  /** The display rows' references (#334): dotted lines, not wires */
  refs: Reference[];
  problems: Problem[];
  canvas: Canvas;
}

/**
 * Edits to the canvas: ports shown (L-T6), boxes placed and folded, groups (L-T7, `apply.ts`); the free nodes' (§5.7:
 * addNode, moveNode, setParams, wire, unwire, run) are declared only
 */
export type CanvasEdit =
  | { edit: "addNode"; node: CanvasNode }
  | { edit: "moveNode"; id: string; x: number; y: number }
  | { edit: "setParams"; id: string; params: Record<string, unknown> }
  | { edit: "wire"; from: [string, string]; to: [string, string] }
  | { edit: "unwire"; from: [string, string]; to: [string, string] }
  | { edit: "run"; id: string }
  /** Show these ports of a box beyond its wired ones (`LogicCanvas.exposed`, L-T6) */
  | { edit: "expose"; box: string; ports: string[] }
  /** Hide the ports nobody wired again – of one box, or of every box at the level (Blender's Ctrl+H) */
  | { edit: "hideUnused"; box?: string }
  /**
   * Pin boxes where they were let go (`LogicCanvas.positions`, schema 5.2 – on the 1rem grid), over the automatic layout; a
   * key `note:<id>` moves that note instead (its own place, or its offset from its box – schema 5.3)
   */
  | { edit: "place"; positions: Record<string, { x: number; y: number }> }
  /** Drop the pins of the level's boxes: the automatic layout places them again (*Tidy*) */
  | { edit: "tidy" }
  /** Fold a box to its head, or open it (`LogicCanvas.folds`) */
  | { edit: "fold"; box: string; folded: boolean }
  /** A new group round these boxes (each leaves its old group – no nesting): `group-N`, labelled `group N`, the first free colour */
  | { edit: "group"; boxes: string[] }
  /** Rename a group in place; an empty label removes the group (its boxes stay) */
  | { edit: "renameGroup"; group: string; label: string }
  /** Remove these groups – their boxes stay where they are (the frame head's menu, Ctrl+Shift+G) */
  | { edit: "ungroup"; groups: string[] }
  /** The group's next colour of the deck's six */
  | { edit: "groupColour"; group: string }
  /** Fold a group to one pill, or open it */
  | { edit: "foldGroup"; group: string; folded: boolean }
  /**
   * A new empty note (schema 5.3, the deck's `Note`): free at `at` (flow coordinates), or beside `box` (it moves with it);
   * `author`: who writes it (the signed-in person, as the editor names them)
   */
  | { edit: "note"; at?: { x: number; y: number }; box?: string; author?: string }
  /** A note's text; an empty text removes the note (the deck's `NoteText`) */
  | { edit: "noteText"; note: string; text: string };

/**
 * Where a new rule goes: a node of a target's tree (its first behaviour component; one is added when it has none), a
 * scene's logic or the experience's
 */
export type RulePlace = { doc: "entity"; target: string; path: number[] } | { doc: "scene"; scene: string } | { doc: "experience" };

/**
 * An edit of the logic as an op on one document (`apply.ts`). `connect` and `disconnect` name the ports and the wire by the
 * level's view (`[box, port]`, an edge's id); the others name the document place (`Home`). `verb`: the verb of the action a
 * `connect` from a rule's `do` to a node makes (show by default).
 */
export type Op = (
  /** `when` / `do`: the new rule comes with a condition and actions (the add menu's action with no rule selected, #334) */
  | { op: "addRule"; at: RulePlace; on?: string; when?: string | null; do?: BehaviourAction[] }
  /** What a rule listens to (a wire dropped from its on on the empty canvas, #334) */
  | { op: "setEvent"; home: Home; on: string }
  /** `event`: which of a plugin's events a wire to its `emits` or `listens` means (its first by default, #334) */
  | { op: "connect"; from: [string, string]; to: [string, string]; verb?: BehaviourAction["action"]; event?: string }
  | { op: "disconnect"; edge: string }
  /** `when`: the condition's expression (null: always) – type-checked before it is written (B8) */
  | { op: "setCondition"; home: Home; when: string | null }
  | { op: "addAction"; home: Home; action: BehaviourAction }
  | { op: "removeAction"; home: Home }
  | { op: "removeRule"; home: Home }
  | { op: "canvas"; edit: CanvasEdit }
) & {
  /**
   * Pin the box the op creates where it was asked for (a wire let go on the empty canvas, a right click there – flow
   * coordinates, schema 5.2): a new rule's box, else the first box new to the level (a variable, an event)
   */
  pin?: { x: number; y: number };
};
