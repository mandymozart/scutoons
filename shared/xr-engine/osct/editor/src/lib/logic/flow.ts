import type { LogicCanvas, LogicCanvasPoint } from "@engine/types";
import { LOGIC_SIZE, layoutLogic, type LayoutSize } from "./layout";
import { neverFires, stubNeverFires } from "./navigation";
import { CARD_HEAD, CARD_W, GRID, GROUP_COLOURS, PIN_ROW, deckSignal, snap, snapUp, type DeckSignal } from "./signals";
import { typeWords } from "./spec-ports";
import type { Context, DisplayRow, GraphNode, PortKind, PortSpec, Signal, Stub, View } from "./types";

/**
 * A level's `View` as boxes and wires to draw (#334 L-T3, L-T7): the rendering layer (`panels/LogicCanvas.svelte`,
 * `@xyflow/svelte` – decision L7) gets positions and ports from here and owns nothing. The boxes come from `layout.ts`; a
 * level's **stubs** sit at its border – what comes in in a column before the first, what goes out in a column after the
 * last – each a box of its own named by its far end. Every box has its ports (handles): the ports its node type gives
 * (`registry.ts`) plus any port an edge names that the type does not (so no wire is dropped), the inlets on the left, the
 * outlets on the right. A wire carries its signal (OSCT's, and the deck's it is drawn in – `signals.ts`, L-T8) and whether
 * it can fire (`navigation.ts`: an event between two scenes never does).
 *
 * **The deck's cards** (L-T7): a box is 14rem wide, its head 2rem; every named port is a **row** of 1.5rem under the head –
 * the inlets on the left, the outlets on the right, row by row – and the box's height snaps to the 1rem grid. Unnamed ports
 * (an event's, a node's `in`, a stub's) and a rule's **+** sit on the head. A spec's ports (L-T6, `spec-ports.ts`) show
 * only while wired or exposed: the hidden ones wait behind the box's **+** (`FlowBox.hidden`). The level's layout takes
 * the tallest box's height (`boxHeight`).
 *
 * **Display rows** (#334, Tilman 2026-11-21): what a plugin's code uses that no wire can change – it reads, sets, drives – is
 * text under the pins, below a thin divider (`reads hasKey, scene`), a 1.25rem row each; each name's box gets a **reference**
 * (`FlowRef`): a fine dotted line from the row to the box, never a wire – nothing selects, deletes or lights it. The layout
 * places along the references as along wires, and the card's height counts the rows.
 *
 * **The canvas** (`LogicCanvas`, schema 5.2): a box the author dragged is **pinned** where it was let go (`positions`) over
 * the layout; a folded box (`folds`) is its head alone. A **group** (as the deck's) is a frame round its boxes at this
 * level – their bounds padded 1rem at the sides and below, 2rem on top for its head – in its colour; folded, it is one
 * **pill** instead of its boxes, and their wires to the rest end on the pill.
 *
 * **Notes** (schema 5.3, the deck's notes): the level's notes (`LogicCanvasNote.view` – a scene's canvas holds its targets'
 * views too) as cards of their own, 12rem wide. A free note stands at its `x`, `y`; a note beside a box at the box's place
 * plus its offset (a folded group's pill stands in for the box), joined to it by a fine dotted line (`FlowNote.link`) – no
 * wire. A note is no logic: the layout, the wires and the issues never see it. A note whose box is not drawn here stands
 * at its `x`, `y` as a free one.
 */

export interface FlowPort {
  id: string;
  label: string;
  signal: Signal;
  /** The deck's signal it is drawn in (`signals.ts`) */
  deck: DeckSignal;
  dir: "in" | "out";
  /** A spec's port */
  kind?: PortKind;
  /** What it takes or gives, in words (`a number 0–1`) – its tooltip */
  detail?: string;
  /** Whose it is (`video`, `hum`) */
  owner?: string;
}

/** A canvas group as drawn: its frame (or its pill, folded) round the members shown at this level */
export interface FlowGroup {
  id: string;
  label: string;
  colour: string;
  folded: boolean;
  /** Its members at this level (box ids) */
  members: string[];
  /** The frame: the members' bounds padded */
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface FlowBox {
  id: string;
  x: number;
  y: number;
  /** The node it draws; null for a stub or a folded group's pill */
  node: GraphNode | null;
  /** The stub it draws */
  stub: Stub | null;
  /** The folded group it draws (a pill) */
  pill?: FlowGroup;
  /** The ports drawn: the grammar's, and a spec's that are wired or exposed */
  ports: FlowPort[];
  /** A spec's ports that are hidden (the box's + exposes them) */
  hidden: FlowPort[];
  /** Its size in px: 14rem wide; the head, and a row per named port, on the grid */
  w: number;
  h: number;
  /** The deck's signal of its kind – its head's tint, its border, its icon (null: muted) */
  colour: DeckSignal | null;
  /** The author dragged it: it stays where it was let go (`LogicCanvas.positions`) */
  pinned: boolean;
  /** Folded to its head (`LogicCanvas.folds`) */
  folded: boolean;
  /** The canvas group it is in */
  group?: string;
  /** Its display rows (a plugin's reads, sets, drives – #334): text, no pins */
  rows: FlowRow[];
}

/** A display row as drawn: its id and its text */
export interface FlowRow {
  id: DisplayRow["id"];
  text: string;
}

/**
 * A reference line (#334): from a box's display row (`sourceHandle` `ref:l:<row>` / `ref:r:<row>`, or the head's `ref:hl` /
 * `ref:hr` when the box is folded) to the box it names (`ref:l` / `ref:r`) – the sides that face each other
 */
export interface FlowRef {
  id: string;
  source: string;
  sourceHandle: string;
  target: string;
  targetHandle: string;
}

export interface FlowWire {
  id: string;
  source: string;
  sourcePort: string;
  target: string;
  targetPort: string;
  label: string;
  signal: Signal;
  /** The deck's signal it is drawn in: its source port's */
  deck: DeckSignal;
  /** It never fires (an event between two scenes, `navigation.ts`) */
  never: boolean;
  /** The view's edge it draws (a stub's edge for a wire at the border): what an op names */
  edge: string;
  /** It runs against the flow (a loop) */
  back: boolean;
}

/** A note as drawn: where it stands (flow px) and, beside a box, the dotted line to it */
export interface FlowNote {
  id: string;
  text: string;
  author?: string;
  x: number;
  y: number;
  /** The box it stands beside, when that box is drawn here */
  box?: string;
  /** Its offset from that box (what the canvas keeps) */
  offset?: { x: number; y: number };
  /** The dotted line to its box: the note's side and the box's that face each other */
  link?: { target: string; sourceHandle: "note:l" | "note:r"; targetHandle: "ref:l" | "ref:r" };
}

export interface Flow {
  boxes: FlowBox[];
  /** The level's notes (schema 5.3) – no logic */
  notes: FlowNote[];
  wires: FlowWire[];
  /** The canvas groups at this level, folded ones included (their pill is a box) */
  groups: FlowGroup[];
  /** The display rows' reference lines – not wires */
  refs: FlowRef[];
}

/** The head of a box and one row of a named port, in px (the deck's 2rem and 1.5rem) */
export const BOX_HEAD = CARD_HEAD;
export const PORT_ROW = PIN_ROW;
/** A frame's padding round its boxes: the sides and below, and on top for its head */
export const FRAME_PAD = GRID;
export const FRAME_HEAD = 2 * GRID;
/** A folded group's pill id */
export const pillId = (group: string): string => `fold:${group}`;
/** A note's node id on the canvas – also how the canvas's selection names it (`note:<id>`) */
export const NOTE_NODE = "note:";
/** A note's width (12rem) and where a new note stands beside its box: right of it, at its head */
export const NOTE_W = 12 * GRID;
export const NOTE_BESIDE = { x: CARD_W + GRID, y: 0 } as const;
/**
 * The view a note of the level's canvas belongs to (`LogicCanvasNote.view`): a target's entity level `<target>` or
 * `<target>/<path>` (`door/0.1`); the scene's and the experience's own level none
 */
export const noteView = (context: Context): string | undefined =>
  context.level === "entity" ? `${context.target}${context.path.length ? `/${context.path.join(".")}` : ""}` : undefined;

/** A port as drawn */
const flowPort = ({ id, label, signal, dir, kind, type, owner }: PortSpec): FlowPort => ({
  id, label, signal, deck: deckSignal(signal, type, kind), dir,
  ...(kind ? { kind, owner, detail: type ? typeWords(type) : kind === "action" ? "a rule's verb" : kind === "event" ? "an event, by name" : undefined } : {}),
});

const portsOf = (spec: readonly PortSpec[], used: readonly FlowPort[]): FlowPort[] => {
  const ports: FlowPort[] = spec.filter(p => !p.hidden).map(flowPort);
  for (const u of used) if (!ports.some(p => p.id === u.id && p.dir === u.dir)) ports.push(u);
  return ports;
};

/** A port drawn as a row with its name (the rest sit on the head: unnamed ones, a rule's +) */
export const isRow = (port: Pick<FlowPort, "id" | "label">): boolean => !!port.label && port.id !== "do";

/** A box's rows: the named inlets on the left and outlets on the right, row by row */
export const rowsOf = (ports: readonly FlowPort[]): { ins: FlowPort[]; outs: FlowPort[]; count: number } => {
  const ins = ports.filter(p => p.dir === "in" && isRow(p));
  const outs = ports.filter(p => p.dir === "out" && isRow(p));
  return { ins, outs, count: Math.max(ins.length, outs.length) };
};

/** A display row's height (1.25rem) and the divider's room above the rows */
export const DISPLAY_ROW = 1.25 * GRID;
export const DIVIDER = GRID / 4;

/** A display row's words: `reads hasKey, scene`, `goes to 3–4` */
export const rowText = (row: DisplayRow): string => `${row.id === "goto" ? "goes to" : row.id} ${row.names.join(", ")}`;

/** A box's height: its head, a row per named port and its display rows, grown to the grid (folded: the head alone) */
export const boxHeight = (ports: readonly FlowPort[], folded = false, displayRows = 0): number => {
  const { count } = rowsOf(ports);
  if (folded || (!count && !displayRows)) return BOX_HEAD;
  return snapUp(BOX_HEAD + count * PORT_ROW + (displayRows ? DIVIDER + displayRows * DISPLAY_ROW : 0) + GRID / 4);
};

/** A handle's id: the port's id with its side, so an inlet and an outlet of one name stay apart */
export const handleId = (port: Pick<FlowPort, "id" | "dir">): string => `${port.dir}:${port.id}`;

/**
 * The deck's colour of a box's kind: the signal of what it gives (its first outlet), else muted – a scene or a group is a
 * scene, a stub or a far box far, a variable its value
 */
export const kindColour = (node: GraphNode | null, ports: readonly FlowPort[]): DeckSignal | null => {
  if (!node || node.far) return "far";
  if (node.kind === "scene" || node.kind === "group") return "scene";
  return ports.find(p => p.dir === "out" && p.id !== "do")?.deck ?? null;
};

/** The frame round boxes: their bounds padded (1rem at the sides and below, 2rem on top for the head) */
export const frameOf = (boxes: readonly { x: number; y: number; w: number; h: number }[]): { x: number; y: number; w: number; h: number } => {
  const left = Math.min(...boxes.map(b => b.x));
  const top = Math.min(...boxes.map(b => b.y));
  const right = Math.max(...boxes.map(b => b.x + b.w));
  const bottom = Math.max(...boxes.map(b => b.y + b.h));
  return { x: left - FRAME_PAD, y: top - FRAME_HEAD, w: right - left + 2 * FRAME_PAD, h: bottom - top + FRAME_HEAD + FRAME_PAD };
};

export const flowOf = (view: View, canvas: LogicCanvas | undefined = undefined, base: LayoutSize = LOGIC_SIZE): Flow => {
  const folds = new Set(canvas?.folds ?? []);
  const positions: Record<string, LogicCanvasPoint> = canvas?.positions ?? {};
  // The tallest box sets the rows' height: a box with exposed ports must not reach into the next one
  const size = { ...base, h: Math.max(base.h, ...view.nodes.map(n => boxHeight(n.ports.filter(p => !p.hidden).map(flowPort), folds.has(n.id), n.rows?.length ?? 0))) };
  // A reference places like a wire: a variable a plugin reads before it, one it sets after it
  const refs = view.refs ?? [];
  const refEdges = refs.map(r => (r.dir === "in" ? { from: r.to, to: r.from[0], label: r.from[1] } : { from: r.from[0], to: r.to, label: r.from[1] }));
  // The boxes without a wire (a scene without crossing logic, a plugin nobody's events touch) are no flow: `layout.ts` would
  // stack them all in its first column, so they sit in rows under the graph instead, in their order
  const wired = new Set([...view.edges.flatMap(e => [e.from[0], e.to[0]]), ...refEdges.flatMap(e => [e.from, e.to]), ...view.stubs.map(s => (s.side === "in" ? s.edge.to[0] : s.edge.from[0]))]);
  const loose = view.nodes.filter(n => !wired.has(n.id)).map(n => n.id);
  const graph = view.nodes.filter(n => wired.has(n.id)).map(n => n.id);
  const flowLayout = layoutLogic(graph, [...view.edges.map(e => ({ from: e.from[0], to: e.to[0], label: e.label })), ...refEdges], size);
  const perRow = Math.max(1, Math.ceil(Math.sqrt(loose.length * 2)), Math.round((flowLayout.width + size.gapX / 2) / (size.w + size.gapX / 2)));
  const below = graph.length ? flowLayout.height + 3 * size.gapY : 0;
  const looseRows = Math.ceil(loose.length / perRow);
  const layout = {
    ...flowLayout,
    nodes: {
      ...flowLayout.nodes,
      ...Object.fromEntries(loose.map((id, i) => [id, { x: (i % perRow) * (size.w + size.gapX / 2), y: below + Math.floor(i / perRow) * (size.h + size.gapY), column: -1 }])),
    },
    width: Math.max(flowLayout.width, Math.min(loose.length, perRow) * (size.w + size.gapX / 2) - size.gapX / 2),
    height: graph.length ? flowLayout.height : looseRows * (size.h + size.gapY) - size.gapY,
  };
  const used = new Map<string, FlowPort[]>();
  const use = (node: string, port: FlowPort) => {
    const list = used.get(node) ?? [];
    if (!list.some(p => p.id === port.id && p.dir === port.dir)) list.push(port);
    used.set(node, list);
  };
  const usedPort = (id: string, signal: Signal, dir: "in" | "out"): FlowPort => ({ id, label: id, signal, deck: deckSignal(signal), dir });

  const backs = new Map(layout.edges.map(e => [`${e.from}>${e.to}`, e.back]));
  const wires: FlowWire[] = view.edges.map(e => {
    use(e.from[0], usedPort(e.from[1], e.signal, "out"));
    use(e.to[0], usedPort(e.to[1], e.signal, "in"));
    return {
      id: e.id, edge: e.id, source: e.from[0], sourcePort: handleId({ id: e.from[1], dir: "out" }), target: e.to[0], targetPort: handleId({ id: e.to[1], dir: "in" }),
      label: e.label, signal: e.signal, deck: deckSignal(e.signal), never: neverFires(e), back: backs.get(`${e.from[0]}>${e.to[0]}`) ?? false,
    };
  });

  // The stubs at the border: one box each, in a column before (in) or after (out) the level's boxes
  const { w, h, gapX, gapY } = size;
  const column = (stubs: Stub[]) => {
    const tall = stubs.length * h + Math.max(0, stubs.length - 1) * gapY;
    const top = (layout.height - tall) / 2;
    return stubs.map((stub, i) => ({ stub, y: top + i * (h + gapY) }));
  };
  const indexed = view.stubs.map((stub, i) => ({ stub, id: `stub:${i}` }));
  const idOf = new Map(indexed.map(s => [s.stub, s.id]));
  const placed = [
    ...column(view.stubs.filter(s => s.side === "in")).map(({ stub, y }) => ({ stub, x: -(w + gapX), y })),
    ...column(view.stubs.filter(s => s.side === "out")).map(({ stub, y }) => ({ stub, x: (layout.width || w) + gapX, y })),
  ];
  const stubBoxes: FlowBox[] = placed.map(({ stub, x, y }) => {
    const id = idOf.get(stub)!;
    const near = stub.side === "in" ? stub.edge.to : stub.edge.from;
    // A stub's one port sits on its head: its name is the stub's own
    const port: FlowPort = { id: stub.side === "in" ? "out" : "in", label: "", signal: stub.edge.signal, deck: deckSignal(stub.edge.signal), dir: stub.side === "in" ? "out" : "in" };
    use(near[0], usedPort(near[1], stub.edge.signal, stub.side === "in" ? "in" : "out"));
    const never = stubNeverFires(stub, view);
    wires.push(stub.side === "in"
      ? { id: `${id}>`, edge: stub.edge.id, source: id, sourcePort: handleId(port), target: near[0], targetPort: handleId({ id: near[1], dir: "in" }), label: stub.edge.label, signal: stub.edge.signal, deck: port.deck, never, back: false }
      : { id: `>${id}`, edge: stub.edge.id, source: near[0], sourcePort: handleId({ id: near[1], dir: "out" }), target: id, targetPort: handleId(port), label: stub.edge.label, signal: stub.edge.signal, deck: port.deck, never, back: false });
    return { id, x: snap(x), y: snap(y), node: null, stub, ports: [port], hidden: [], w: CARD_W, h: BOX_HEAD, colour: "far", pinned: false, folded: false, rows: [] };
  });

  let boxes: FlowBox[] = view.nodes.map(node => {
    const at = positions[node.id] ?? layout.nodes[node.id];
    const ports = portsOf(node.ports, used.get(node.id) ?? []);
    const folded = folds.has(node.id);
    const rows = (node.rows ?? []).map(row => ({ id: row.id, text: rowText(row) }));
    return {
      id: node.id, x: snap(at.x), y: snap(at.y), node, stub: null, ports, hidden: node.ports.filter(p => p.hidden).map(flowPort),
      w: CARD_W, h: boxHeight(ports, folded, rows.length), colour: kindColour(node, ports), pinned: !!positions[node.id], folded, rows,
    };
  });
  // A wire's colour is its source port's (a value's type: a number control, true / false a vote …)
  const portAt = new Map([...boxes, ...stubBoxes].flatMap(b => b.ports.map(p => [`${b.id}|${handleId(p)}`, p] as const)));
  for (const wire of wires) wire.deck = portAt.get(`${wire.source}|${wire.sourcePort}`)?.deck ?? wire.deck;

  // The canvas groups at this level: a box sits in one group (the first that names it)
  const groups: FlowGroup[] = [];
  const memberOf = new Map<string, string>();
  const at = new Map(boxes.map(b => [b.id, b]));
  (canvas?.groups ?? []).forEach((group, i) => {
    const members = group.members.filter(id => at.has(id) && !memberOf.has(id));
    if (!members.length) return;
    members.forEach(id => memberOf.set(id, group.id));
    const frame = frameOf(members.map(id => at.get(id)!));
    groups.push({ id: group.id, label: group.label, colour: group.colour ?? GROUP_COLOURS[i % GROUP_COLOURS.length], folded: !!group.folded, members, ...frame });
  });
  boxes = boxes.map(b => (memberOf.has(b.id) ? { ...b, group: memberOf.get(b.id) } : b));

  // A folded group is one pill where its boxes were: their wires to the rest end on it, the ones between them go
  const pills: FlowBox[] = [];
  const pillOf = new Map<string, string>();
  for (const group of groups.filter(g => g.folded)) {
    const id = pillId(group.id);
    group.members.forEach(m => pillOf.set(m, id));
    pills.push({
      id, x: group.x + FRAME_PAD, y: group.y + FRAME_HEAD, node: null, stub: null, pill: group, hidden: [], w: CARD_W, h: BOX_HEAD, colour: null, pinned: false, folded: true, rows: [],
      ports: [{ id: "in", label: "", signal: "event", deck: "event", dir: "in" }, { id: "out", label: "", signal: "event", deck: "event", dir: "out" }],
    });
  }
  const shownWires = !pillOf.size ? wires : wires.flatMap(wire => {
    const source = pillOf.get(wire.source);
    const target = pillOf.get(wire.target);
    if (source && source === target) return [];
    return [{ ...wire, ...(source ? { source, sourcePort: "out:out" } : {}), ...(target ? { target, targetPort: "in:in" } : {}) }];
  });
  const refKey = (r: FlowRef) => `${r.source}|${r.sourceHandle}|${r.target}`;
  // The references: from the row (the folded box's or the pill's head) to the box, on the sides that face each other
  const shown = new Map([...boxes, ...pills].map(b => [b.id, b]));
  const flowRefs: FlowRef[] = refs.flatMap(r => {
    const source = pillOf.get(r.from[0]) ?? r.from[0];
    const target = pillOf.get(r.to) ?? r.to;
    const [a, b] = [shown.get(source), shown.get(target)];
    if (!a || !b || source === target) return [];
    const right = b.x + b.w / 2 >= a.x + a.w / 2;
    const onRow = source === r.from[0] && !a.folded && a.rows.some(row => row.id === r.from[1]);
    return [{
      id: r.id, source, target, targetHandle: right ? "ref:l" : "ref:r",
      sourceHandle: onRow ? `ref:${right ? "r" : "l"}:${r.from[1]}` : right ? "ref:hr" : "ref:hl",
    }];
  });
  // The notes of this level's view: a free one where it stands, one beside a box at the box (or its group's pill)
  const here = noteView(view.context);
  const notes: FlowNote[] = (canvas?.notes ?? []).filter(n => n.view === here).map(note => {
    const drawn = note.box ? shown.get(pillOf.get(note.box) ?? note.box) : undefined;
    const base = { id: note.id, text: note.text, ...(note.author ? { author: note.author } : {}) };
    if (!drawn) return { ...base, x: note.x, y: note.y };
    const x = drawn.x + note.x;
    const right = x + NOTE_W / 2 >= drawn.x + drawn.w / 2;
    return {
      ...base, x, y: drawn.y + note.y, box: drawn.id, offset: { x: note.x, y: note.y },
      link: { target: drawn.id, sourceHandle: right ? "note:l" : "note:r", targetHandle: right ? "ref:r" : "ref:l" },
    };
  });
  return { boxes: [...boxes.filter(b => !pillOf.has(b.id)), ...pills, ...stubBoxes], notes, wires: shownWires, groups, refs: flowRefs.filter((r, i) => flowRefs.findIndex(q => refKey(q) === refKey(r)) === i) };
};
