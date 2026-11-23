/**
 * Where the Logic panel draws a level's logic graph (#239, #334 – `project.ts`): a small **layered layout, left to right**
 * – no dependency. Cycles (a rule that emits what another listens to, a rule that plays its own node) are broken by turning
 * the edges back that a depth-first walk finds closing a loop; each node's column is its longest path from a source;
 * within a column the nodes are ordered by the mean place of their neighbours (a few sweeps both ways), then each
 * column is centred. A forward edge is a curve from the right side of its source to the left side of its target; a
 * back edge loops below both. Pure: sizes in px, the panel scales the drawing.
 */

export interface LayoutSize {
  /** A node's box */
  w: number;
  h: number;
  /** Between columns and between the boxes of a column */
  gapX: number;
  gapY: number;
}

/** The deck's card (14rem × 2rem) and gaps on the 1rem grid (#334 L-T7) */
export const LOGIC_SIZE: LayoutSize = { w: 224, h: 32, gapX: 96, gapY: 16 };

export interface PlacedNode {
  x: number;
  y: number;
  column: number;
}

export interface RoutedEdge {
  from: string;
  to: string;
  label: string;
  /** The SVG path */
  d: string;
  /** Where its label sits */
  lx: number;
  ly: number;
  /** It runs against the flow (a loop) */
  back: boolean;
}

export interface LogicLayout {
  nodes: Record<string, PlacedNode>;
  edges: RoutedEdge[];
  width: number;
  height: number;
}

/** The edges a depth-first walk finds closing a loop, by index – turned back, the rest is acyclic */
const backEdges = (ids: readonly string[], edges: readonly { from: string; to: string }[]): Set<number> => {
  const out = new Map<string, number[]>(ids.map(id => [id, []]));
  edges.forEach((e, i) => out.get(e.from)?.push(i));
  const state = new Map<string, 1 | 2>();
  const back = new Set<number>();
  const visit = (id: string) => {
    state.set(id, 1);
    for (const i of out.get(id) ?? []) {
      const next = edges[i].to;
      if (state.get(next) === 1 || next === id) back.add(i);
      else if (!state.has(next)) visit(next);
    }
    state.set(id, 2);
  };
  ids.forEach(id => { if (!state.has(id)) visit(id); });
  return back;
};

export const layoutLogic = (ids: readonly string[], edges: readonly { from: string; to: string; label: string }[], size: LayoutSize = LOGIC_SIZE): LogicLayout => {
  const known = new Set(ids);
  const valid = edges.filter(e => known.has(e.from) && known.has(e.to));
  const back = backEdges(ids, valid);
  const forward = valid.filter((e, i) => !back.has(i) && e.from !== e.to);

  // Columns: the longest path from a source (Kahn's order over the forward edges)
  const column = new Map<string, number>(ids.map(id => [id, 0]));
  const incoming = new Map<string, number>(ids.map(id => [id, 0]));
  forward.forEach(e => incoming.set(e.to, (incoming.get(e.to) ?? 0) + 1));
  const queue = ids.filter(id => incoming.get(id) === 0);
  while (queue.length) {
    const id = queue.shift()!;
    for (const e of forward.filter(f => f.from === id)) {
      column.set(e.to, Math.max(column.get(e.to)!, column.get(id)! + 1));
      incoming.set(e.to, incoming.get(e.to)! - 1);
      if (incoming.get(e.to) === 0) queue.push(e.to);
    }
  }

  // Order within the columns: the mean place of the neighbours in the column before (then after), a few sweeps
  const columns: string[][] = [];
  ids.forEach(id => (columns[column.get(id)!] ??= []).push(id));
  for (let c = 0; c < columns.length; c++) columns[c] ??= [];
  const place = () => new Map(columns.flatMap(col => col.map((id, i) => [id, i] as const)));
  const sweep = (c: number, neighbours: (id: string) => string[]) => {
    const at = place();
    const mean = (id: string) => {
      const ns = neighbours(id).map(n => at.get(n)!).filter(n => n !== undefined);
      return ns.length ? ns.reduce((a, b) => a + b, 0) / ns.length : at.get(id)!;
    };
    columns[c] = [...columns[c]].map(id => ({ id, m: mean(id) })).sort((a, b) => a.m - b.m).map(x => x.id);
  };
  const before = (id: string) => forward.filter(e => e.to === id).map(e => e.from);
  const after = (id: string) => forward.filter(e => e.from === id).map(e => e.to);
  for (let pass = 0; pass < 4; pass++) {
    for (let c = 1; c < columns.length; c++) sweep(c, before);
    for (let c = columns.length - 2; c >= 0; c--) sweep(c, after);
  }

  // Coordinates: each column centred on the tallest
  const { w, h, gapX, gapY } = size;
  const tallest = Math.max(0, ...columns.map(col => col.length));
  const height = tallest ? tallest * h + (tallest - 1) * gapY : 0;
  const nodes: Record<string, PlacedNode> = {};
  columns.forEach((col, c) => {
    const top = (height - (col.length * h + (col.length - 1) * gapY)) / 2;
    col.forEach((id, i) => { nodes[id] = { x: c * (w + gapX), y: top + i * (h + gapY), column: c }; });
  });
  const width = columns.length ? columns.length * w + (columns.length - 1) * gapX : 0;

  // Edges: forward curves right → left; a loop below both boxes (the drawing grows by the room it needs)
  const loop = gapY + h / 2;
  const routed: RoutedEdge[] = valid.map(e => {
    const s = nodes[e.from];
    const t = nodes[e.to];
    const ax = s.x + w;
    const ay = s.y + h / 2;
    const bx = t.x;
    const by = t.y + h / 2;
    if (t.column > s.column) {
      const dx = (bx - ax) / 2;
      return { ...e, back: false, d: `M${ax},${ay} C${ax + dx},${ay} ${bx - dx},${by} ${bx},${by}`, lx: (ax + bx) / 2, ly: (ay + by) / 2 };
    }
    const low = Math.max(s.y, t.y) + h + loop;
    const reach = gapX / 2;
    return {
      ...e, back: true,
      d: `M${ax},${ay} C${ax + reach},${ay} ${ax + reach},${low} ${(ax + bx) / 2},${low} C${bx - reach},${low} ${bx - reach},${by} ${bx},${by}`,
      lx: (ax + bx) / 2, ly: low,
    };
  });
  const lowest = Math.max(height, ...routed.filter(r => r.back).map(r => r.ly + gapY));
  return { nodes, edges: routed, width, height: lowest };
};
