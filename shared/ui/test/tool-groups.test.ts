import { describe, it } from "node:test";
import { expect } from "./expect.ts";
import { activeIndex, activeOption, closesOnResize, expandedGroups, groupWidth, menuPlace, menuStep, openFocus, toolbarWidth, type GroupShape, type ToolbarMetrics } from "../src/editor/tool-groups.ts";

const shading = [
  { value: "wireframe", name: "Wire" },
  { value: "solid", name: "Solid" },
  { value: "textured", name: "Textured" },
  { value: "rendered", name: "Rendered" },
];

describe("a tool group's active option", () => {
  it("is the option with the value, its icon shows on the group's button", () => {
    expect(activeOption(shading, "textured").name).toBe("Textured");
    expect(activeIndex(shading, "textured")).toBe(2);
  });

  it("falls back to the first option for an unknown value", () => {
    expect(activeOption(shading, "nope").name).toBe("Wire");
    expect(activeIndex(shading, "nope")).toBe(0);
  });
});

describe("the group menu's keyboard (WAI-ARIA menu)", () => {
  it("moves with the arrows and wraps around", () => {
    expect(menuStep("ArrowDown", 0, 3)).toBe(1);
    expect(menuStep("ArrowRight", 2, 3)).toBe(0);
    expect(menuStep("ArrowUp", 0, 3)).toBe(2);
    expect(menuStep("ArrowLeft", 2, 3)).toBe(1);
  });

  it("jumps with Home and End", () => {
    expect(menuStep("Home", 2, 4)).toBe(0);
    expect(menuStep("End", 0, 4)).toBe(3);
  });

  it("leaves every other key (Enter, Esc, W …) to the menu's buttons and the page", () => {
    expect(menuStep("Enter", 1, 3)).toBeNull();
    expect(menuStep("Escape", 1, 3)).toBeNull();
    expect(menuStep("w", 1, 3)).toBeNull();
    expect(menuStep("ArrowDown", 0, 0)).toBeNull();
  });
});

describe("which groups stay expanded: decided by the toolbar's room, not the device", () => {
  // The toolbar's CSS at 16 px: gap .125rem, a rule of .375rem + 1px, padding .125rem and a 1px border
  const at = (button: number): ToolbarMetrics => ({ button, gap: 2, rule: 7, chrome: 6 });
  const fine = at(28);
  const finger = at(44);
  const groups: GroupShape[] = [{ options: 3 }, { options: 3 }, { options: 4, tight: true }];

  it("measures a group collapsed (one button) and expanded (its buttons and gaps; the shading without)", () => {
    expect(groupWidth(groups[0], false, fine)).toBe(28);
    expect(groupWidth(groups[0], true, fine)).toBe(3 * 28 + 2 * 2);
    expect(groupWidth(groups[2], true, fine)).toBe(4 * 28);
    expect(toolbarWidth(groups, [false, false, false], finger)).toBe(6 + 3 * 44 + 2 * (2 + 7));
  });

  it("keeps every group expanded on a desktop's stage", () => {
    expect(expandedGroups(groups, 600, fine)).toEqual([true, true, true]);
    // exactly the room the expanded toolbar needs
    const all = toolbarWidth(groups, [true, true, true], fine);
    expect(expandedGroups(groups, all, fine)).toEqual([true, true, true]);
    expect(expandedGroups(groups, all - 1, fine)).toEqual([true, true, false]);
  });

  it("collapses the groups from the last on as the room shrinks", () => {
    const two = toolbarWidth(groups, [true, true, false], finger);
    const one = toolbarWidth(groups, [true, false, false], finger);
    expect(expandedGroups(groups, two, finger)).toEqual([true, true, false]);
    expect(expandedGroups(groups, two - 1, finger)).toEqual([true, false, false]);
    expect(expandedGroups(groups, one - 1, finger)).toEqual([false, false, false]);
  });

  it("collapses every group on a 360 px phone beside ‹ ›, and the toolbar fits one row there", () => {
    // the stage 360 px less 1rem margins, ‹ › (2 × 44 px and their margins) and the slack
    const room = 360 - 16 - (2 * 44 + 6) - 4;
    expect(expandedGroups(groups, room, finger)).toEqual([false, false, false]);
    expect(toolbarWidth(groups, [false, false, false], finger)).toBeLessThanOrEqual(room);
  });

  it("collapses everything in a room too small for anything", () => {
    expect(expandedGroups(groups, 10, finger)).toEqual([false, false, false]);
  });
});

describe("a toolbar panel on a phone", () => {
  it("opened by a tap focuses the panel, not a field – no keyboard pops up", () => {
    expect(openFocus(false)).toBe("panel");
    expect(openFocus(true)).toBe("first");
  });
  it("stays open when only the height changes (the keyboard, the address bar)", () => {
    expect(closesOnResize(360, 360)).toBe(false);
    expect(closesOnResize(360, 780)).toBe(true);
  });
});

// Where a toolbar button's menu opens: below in a row, beside it in a column, always on the screen
describe("menuPlace", () => {
  const screen = { width: 1000, height: 800 };
  const box = (left: number, top: number, size = 28) => ({ left, top, right: left + size, bottom: top + size, width: size, height: size });
  const menu = { width: 200, height: 300 };

  it("opens below the button in a row, above it with no room below", () => {
    expect(menuPlace(box(100, 50), menu, screen, false)).toEqual({ left: 100, top: 80 });
    expect(menuPlace(box(100, 700), menu, screen, false)).toEqual({ left: 100, top: 398 });
  });

  it("opens beside the button in a column – right of it, left with no room on the right", () => {
    expect(menuPlace(box(20, 100), menu, screen, true)).toEqual({ left: 50, top: 100 });
    expect(menuPlace(box(900, 100), menu, screen, true)).toEqual({ left: 698, top: 100 });
  });

  it("keeps a column's menu on the screen at its lower edge", () => {
    expect(menuPlace(box(20, 700), menu, screen, true)).toEqual({ left: 50, top: 496 });
  });
});
