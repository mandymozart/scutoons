/**
 * The Logic panel's issues bar (#334, Tilman 2026-11-21: "like the issues bar in the scutoons deck editor, more minimal …
 * expand when clicked to show details"). The deck's pattern (`deck/editor/src/module/problems.rs`, `kit/src/editor/status.rs`):
 * errors first, then warnings, each with its place; the same message once; the counts in words ("2 errors · 1 warning").
 * Here a row's place is the box's name and its text what the box does not already say; a click goes to the box.
 */
import type { Context, Problem, Severity } from "./types";

export interface IssueRow {
  severity: Severity;
  /** The box's name (or the rule's place), "" for none */
  place: string;
  /** The short words beside the place */
  text: string;
  /** The whole sentence (the row's title) */
  message: string;
  /** Where a click goes: the box to mark, and the level it is drawn at when that is another one */
  node: string | null;
  context: Context | null;
}

/** The message without the place it starts with ("doorOpened is emitted …" → "is emitted …"), when no short words came */
const withoutPlace = (message: string, place: string): string => {
  if (!place || !message.startsWith(place)) return message;
  const rest = message.slice(place.length).replace(/^[:,]?\s+/, "");
  return rest || message;
};

export const issueRows = (problems: readonly Problem[]): IssueRow[] => {
  const rows = problems.map((p): IssueRow => {
    const place = p.place ?? "";
    return {
      severity: p.severity,
      place,
      text: (p.short ?? withoutPlace(p.message, place)).replace(/\.$/, ""),
      message: p.message,
      node: p.at?.node ?? p.node ?? null,
      context: p.at?.context ?? null,
    };
  });
  // errors first, each kind in its order (a stable sort)
  return rows.filter(r => r.severity === "error").concat(rows.filter(r => r.severity === "warning"));
};

/** The counts in words, the deck's: "2 errors · 1 warning", "1 warning", "no issues" */
export const issueSummary = (rows: readonly IssueRow[]): string => {
  const errors = rows.filter(r => r.severity === "error").length;
  const warnings = rows.length - errors;
  const n = (k: number, word: string) => `${k} ${word}${k === 1 ? "" : "s"}`;
  if (!errors && !warnings) return "no issues";
  if (!warnings) return n(errors, "error");
  if (!errors) return n(warnings, "warning");
  return `${n(errors, "error")} · ${n(warnings, "warning")}`;
};

const OPEN_KEY = "osct.logic.issues.open";

/** The bar's open state, per browser session (it may be blocked: then the bar starts closed) */
export const issuesOpen = {
  read(): boolean {
    try {
      return window.sessionStorage.getItem(OPEN_KEY) === "1";
    } catch {
      return false;
    }
  },
  write(open: boolean): void {
    try {
      window.sessionStorage.setItem(OPEN_KEY, open ? "1" : "0");
    } catch {
      // storage blocked: the state lasts as long as the panel
    }
  },
};
