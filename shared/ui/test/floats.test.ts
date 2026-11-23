import { describe, it } from "node:test";
import { expect } from "./expect.ts";
import {
  at, clampInto, crowd, defaultPlace, isPlace, layout, moved, placeOf, raise, readPlaces, resized, withPlaces,
  type Corner, type Metrics, type PanelSpec, type Place, type Rect,
} from "../src/editor/floats.ts";

// the floating panels' placement (after the SCUTOONS deck editor's floats)
const M: Metrics = { rem: 16, gap: 8, head: 36, button: 28, char: 8 };
const AREA: Rect = { x: 8, y: 8, w: 1424, h: 800 };
const SPECS: PanelSpec[] = [
  { id: "tree", name: "Tree", icon: "tree", about: "", place: at("top-left", 0, 0), size: { w: 16, h: "fill" }, min: { w: 12, h: 8 } },
  { id: "inspector", name: "Inspector", icon: "inspector", about: "", place: at("top-right", 0, 0), size: { w: 25, h: "fill" }, min: { w: 16, h: 10 }, essential: true },
  { id: "scenes", name: "Scenes", icon: "film", about: "", place: at("bottom-left", 16.5, 0), size: { w: "fill", h: 12 }, min: { w: 20, h: 10 } },
  { id: "info", name: "Info", icon: "info", about: "", place: at("bottom-left", 0, 0, { collapsed: true }), size: { w: 16, h: 10 }, min: { w: 12, h: 5 } },
  { id: "tools", name: "Tools", icon: "tool", about: "", place: at("top-left", 16.5, 0), size: { w: "content", h: "content" }, min: { w: 2, h: 2 }, essential: true, inline: true },
];
const ALL = new Set(SPECS.map(s => s.id));
const right = (r: Rect) => r.x + r.w;
const bottom = (r: Rect) => r.y + r.h;
const lay = (places = {}, more: Partial<Parameters<typeof layout>[0]> = {}) =>
  layout({ specs: SPECS, places, shown: ALL, area: AREA, m: M, measured: { tools: { w: 300, h: 28 } }, ...more });

describe("the floating panels: the default layout", () => {
  const views = lay();

  it("anchors each panel to its corner: the tree top left, the inspector right, the scenes at the bottom between them", () => {
    expect(views.tree.rect).toMatchObject({ x: 8, y: 8, w: 256 });
    expect(right(views.inspector.rect)).toBe(right(AREA));
    expect(views.inspector.rect).toMatchObject({ y: 8, w: 400, h: 800 });
    expect(views.scenes.rect).toMatchObject({ x: 8 + 264, h: 192 });
    expect(bottom(views.scenes.rect)).toBe(bottom(AREA));
    // The scenes fill the room between the tree and the inspector, the gap away from both
    expect(right(views.scenes.rect)).toBe(views.inspector.rect.x - 8);
  });

  it("folds the info to its pill at the bottom left; the tree leaves it its room", () => {
    expect(views.info.collapsed).toBe(true);
    expect(views.info.rect).toMatchObject({ x: 8, w: 72 + 32 + 28, h: 36 });
    expect(bottom(views.info.rect)).toBe(bottom(AREA));
    expect(bottom(views.tree.rect)).toBe(views.info.rect.y - 8);
  });

  it("sizes the inline toolbar after its content, beside the tree over the stage", () => {
    expect(views.tools.rect).toMatchObject({ x: 8 + 264, y: 8 });
    expect(views.tools.rect.w).toBe(300 + 36 + 28 + 8);
  });

  it("a toolbar never folds – a fold stored by an older editor is ignored", () => {
    const folded = lay({ tools: { corner: "top-left", dx: 264, dy: 0, collapsed: true } });
    expect(folded.tools.collapsed).toBe(false);
    expect(folded.tools.rect.w).toBe(300 + 36 + 28 + 8);
  });

  it("a vertical toolbar is a column: as wide as its content, its grip and button along it", () => {
    const column = lay({ tools: { corner: "top-left", dx: 264, dy: 0, vertical: true } }, { measured: { tools: { w: 28, h: 300 } } });
    expect(column.tools.rect.w).toBe(28 + 36 - 28 + 2);
    expect(column.tools.rect.h).toBe(300 + 36 + 28 + 8);
  });

  it("lets no two panels come closer than the gap", () => {
    const rects = Object.values(views).map(v => v.rect);
    rects.forEach((a, i) => rects.slice(i + 1).forEach(b => expect(crowd(a, b, 8)).toBe(false)));
  });

  it("an open info panel takes its room from the tree, which comes first", () => {
    const opened = lay({ info: { corner: "bottom-left", dx: 0, dy: 0 } });
    expect(opened.info.rect.h).toBe(160);
    expect(bottom(opened.tree.rect)).toBe(opened.info.rect.y - 8);
  });

  it("leaves out closed panels and panels the page does not draw; the stage takes their room", () => {
    const closed = lay({ tree: { corner: "top-left", dx: 0, dy: 0, closed: true } });
    expect(closed.tree).toBeUndefined();
    const notShown = lay({}, { shown: new Set(["inspector", "scenes"]) });
    expect(Object.keys(notShown).sort()).toEqual(["inspector", "scenes"]);
  });

  it("asks: the Code tab widens the inspector until it was resized", () => {
    expect(lay({}, { asks: { inspector: { w: 36, h: "fill" } } }).inspector.rect.w).toBe(576);
    const resizedOne = { inspector: { corner: "top-right" as const, dx: 0, dy: 0, size: { w: 300, h: 500 } } };
    expect(lay(resizedOne, { asks: { inspector: { w: 36, h: "fill" } } }).inspector.rect).toMatchObject({ w: 300, h: 500 });
  });

  it("keeps every panel inside a small window", () => {
    const small = { x: 8, y: 8, w: 600, h: 400 };
    const views = lay({}, { area: small });
    for (const v of Object.values(views)) {
      expect(v.rect.x).toBeGreaterThanOrEqual(small.x);
      expect(right(v.rect)).toBeLessThanOrEqual(right(small) + 0.01);
      expect(bottom(v.rect)).toBeLessThanOrEqual(bottom(small) + 0.01);
    }
  });
});

describe("the floating panels: places", () => {
  it("keeps a place by the corner nearest to the panel's middle", () => {
    expect(placeOf({ x: 1000, y: 600, w: 200, h: 100 }, AREA)).toEqual({ corner: "bottom-right", dx: 232, dy: 108 });
    expect(placeOf({ x: 50, y: 20, w: 200, h: 100 }, AREA)).toEqual({ corner: "top-left", dx: 42, dy: 12 });
    // fractional: kept as it is, so the rectangle comes back exactly (a rounded one could crowd a neighbour by a pixel)
    expect(placeOf({ x: 50.25, y: 20.5, w: 200, h: 100 }, AREA)).toEqual({ corner: "top-left", dx: 42.25, dy: 12.5 });
  });

  it("comes back at the same distance from its corner when the window changes size", () => {
    const place = { inspector: { corner: "top-right" as const, dx: 40, dy: 20 } };
    const wide = lay(place).inspector.rect;
    const narrow = lay(place, { area: { ...AREA, w: 1100 } }).inspector.rect;
    expect(right(AREA) - right(wide)).toBe(40);
    expect(8 + 1100 - right(narrow)).toBe(40);
  });

  it("the default place is the spec's, in px; an essential panel never starts closed", () => {
    expect(defaultPlace(SPECS[2], M)).toEqual({ corner: "bottom-left", dx: 264, dy: 0 });
    expect(defaultPlace({ ...SPECS[1], place: at("top-right", 0, 0, { closed: true }) }, M)).toEqual({ corner: "top-right", dx: 0, dy: 0 });
  });
});

describe("the floating panels: move and resize, no snap", () => {
  it("a moved panel stays inside the walls", () => {
    expect(moved({ x: 100, y: 100, w: 200, h: 100 }, -500, 20, AREA)).toEqual({ x: 8, y: 120, w: 200, h: 100 });
    expect(clampInto({ x: 1400, y: 790, w: 200, h: 100 }, AREA)).toEqual({ x: 1232, y: 708, w: 200, h: 100 });
  });

  it("resizes by an edge or a corner, never below the least size or past a wall", () => {
    const start = { x: 100, y: 100, w: 300, h: 200 };
    expect(resized(start, { x: 1, y: 0 }, 50, 999, { w: 100, h: 50 }, AREA)).toEqual({ x: 100, y: 100, w: 350, h: 200 });
    expect(resized(start, { x: -1, y: -1 }, 250, 300, { w: 100, h: 50 }, AREA)).toEqual({ x: 300, y: 250, w: 100, h: 50 });
    expect(resized(start, { x: -1, y: 1 }, -500, 2000, { w: 100, h: 50 }, AREA)).toEqual({ x: 8, y: 100, w: 392, h: 708 });
  });

  it("let go, a panel comes back exactly where it was dropped – a few px from a wall or a neighbour it does not snap", () => {
    const dropped = { x: 15, y: 412, w: 256, h: 160 };
    const views = lay({ info: placeOf(dropped, AREA) });
    expect(views.info.rect).toEqual(dropped);
    const beside = { x: views.tree.rect.x + views.tree.rect.w + 3, y: 20, w: 256, h: 160 };
    expect(lay({ info: placeOf(beside, AREA) }).info.rect).toEqual(beside);
  });

  it("the panel touched last lies in front", () => {
    expect(raise(["tree", "inspector", "tools"], "tree")).toEqual(["inspector", "tools", "tree"]);
  });

  it("a panel in the hand lies where the pointer has it; the others stay as they are", () => {
    const held = { id: "tree", rect: { x: 500, y: 300, w: 256, h: 300 } };
    const views = lay({}, { held });
    expect(views.tree.rect).toEqual(held.rect);
    const still = lay();
    for (const id of ["inspector", "scenes", "info", "tools"]) expect(views[id]).toEqual(still[id]);
  });
});

// panels float over the stage: they may overlap, and nothing a panel does moves or shrinks another
describe("the floating panels: overlap, never push", () => {
  const still = lay();

  it("a panel let go on another lies over it; neither moves nor changes size", () => {
    const over = { corner: "top-right" as const, dx: 100, dy: 40 };
    const views = lay({ info: over });
    expect(views.info.rect).toEqual({ x: right(AREA) - 100 - 256, y: 48, w: 256, h: 160 });
    expect(crowd(views.info.rect, views.inspector.rect, 0)).toBe(true);
    expect(views.inspector).toEqual(still.inspector);
    expect(views.scenes).toEqual(still.scenes);
  });

  it("a resized panel keeps its size over its neighbours; they stay where they were", () => {
    const wide = { inspector: { corner: "top-right" as const, dx: 0, dy: 0, size: { w: 1000, h: 800 } } };
    const views = lay(wide);
    expect(views.inspector.rect).toEqual({ x: right(AREA) - 1000, y: 8, w: 1000, h: 800 });
    expect(views.tree).toEqual(still.tree);
    expect(views.tools).toEqual(still.tools);
    expect(views.info).toEqual(still.info);
  });

  it("a fill dimension stops before a panel clear of it, not at one lying over it", () => {
    // the folded info at the bottom left: the tree ends the gap above it
    expect(bottom(still.tree.rect)).toBe(still.info.rect.y - 8);
    // the info opened over the tree: the tree runs on to the wall under it
    const over = lay({ info: { corner: "top-left", dx: 40, dy: 40 } });
    expect(over.tree.rect).toMatchObject({ x: 8, y: 8, h: 800 });
    expect(over.info.rect).toEqual({ x: 48, y: 48, w: 256, h: 160 });
  });

  it("two panels at the same default place do not start on each other; placed there by the person, they may", () => {
    const twin: PanelSpec = { id: "notes", name: "Notes", icon: "info", about: "", place: at("bottom-left", 0, 0, { collapsed: true }), size: { w: 16, h: 10 }, min: { w: 12, h: 5 } };
    const specs = [...SPECS, twin];
    const shown = new Set(specs.map(s => s.id));
    const fresh = layout({ specs, places: {}, shown, area: AREA, m: M, measured: { tools: { w: 300, h: 28 } } });
    expect(crowd(fresh.notes.rect, fresh.info.rect, 8)).toBe(false);
    const placed = layout({ specs, places: { notes: { corner: "bottom-left", dx: 0, dy: 0, collapsed: true } }, shown, area: AREA, m: M, measured: { tools: { w: 300, h: 28 } } });
    expect(placed.notes.rect).toMatchObject({ x: 8, y: bottom(AREA) - 36 });
    expect(placed.info).toEqual(fresh.info);
  });

  it("keeps every panel inside the walls at its asked size and never below its least (random places)", () => {
    let seed = 11;
    const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
    const corners: Corner[] = ["top-left", "top-right", "bottom-left", "bottom-right"];
    for (let round = 0; round < 300; round++) {
      const places: Record<string, Place> = {};
      for (const spec of SPECS) {
        if (rnd() < 0.2) continue;
        const place: Place = { corner: corners[Math.floor(rnd() * 4)], dx: Math.floor(rnd() * 1200), dy: Math.floor(rnd() * 700) };
        if (rnd() < 0.4 && !spec.inline) place.size = { w: 100 + Math.floor(rnd() * 1600), h: 80 + Math.floor(rnd() * 900) };
        if (rnd() < 0.2) place.collapsed = true;
        places[spec.id] = place;
      }
      const views = lay(places);
      for (const [id, v] of Object.entries(views)) {
        const r = v.rect;
        expect(r.x).toBeGreaterThanOrEqual(AREA.x - 0.01);
        expect(r.y).toBeGreaterThanOrEqual(AREA.y - 0.01);
        expect(right(r)).toBeLessThanOrEqual(right(AREA) + 0.01);
        expect(bottom(r)).toBeLessThanOrEqual(bottom(AREA) + 0.01);
        const spec = SPECS.find(s => s.id === id)!;
        const size = places[id]?.size;
        if (size && !v.collapsed) expect([r.w, r.h]).toEqual([Math.min(size.w, AREA.w), Math.min(size.h, AREA.h)]);
        if (!v.collapsed && !spec.inline) {
          expect(r.w).toBeGreaterThanOrEqual(Math.min(spec.min.w * M.rem, size?.w ?? Infinity) - 0.01);
          expect(r.h).toBeGreaterThanOrEqual(Math.min(spec.min.h * M.rem, size?.h ?? Infinity) - 0.01);
        }
      }
    }
  });
});

describe("the floating panels: the layout in this browser", () => {
  it("reads the places of one workspace, only valid ones of known panels; an essential panel is never closed", () => {
    const stored = {
      scene: {
        tree: { corner: "top-left", dx: 10, dy: 0, collapsed: true },
        inspector: { corner: "top-right", dx: 0, dy: 0, closed: true },
        scenes: { corner: "middle", dx: 0, dy: 0 },
        ghost: { corner: "top-left", dx: 0, dy: 0 },
      },
      book: { tree: { corner: "top-left", dx: 99, dy: 0 } },
    };
    expect(readPlaces(stored, "scene", SPECS)).toEqual({
      tree: { corner: "top-left", dx: 10, dy: 0, collapsed: true },
      inspector: { corner: "top-right", dx: 0, dy: 0, closed: false },
    });
    expect(readPlaces("nonsense", "scene", SPECS)).toEqual({});
    expect(isPlace({ corner: "top-left", dx: 0, dy: 0, size: { w: 10, h: -1 } })).toBe(false);
  });

  it("writes one workspace's places and keeps the others; a reset removes that workspace", () => {
    const stored = { book: { tree: { corner: "top-left", dx: 1, dy: 0 } } };
    const scene = { tree: { corner: "top-right" as const, dx: 0, dy: 0 } };
    expect(withPlaces(stored, "scene", scene)).toEqual({ ...stored, scene });
    expect(withPlaces({ ...stored, scene }, "scene", null)).toEqual(stored);
  });
});
