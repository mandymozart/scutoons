// The storyboard page's steps (#123, owner 2026-10-19 – research/STUDIO-LAYOUT.md §6, workflows/storyboard-studio
// README "The layout"): one layout that always looks the same – the shot list, the main view, the dock – and per step
// only which panels are open and the Shot inspector's tab. Each step remembers its own (localStorage
// `scutoons_creator:storyboard:step:<id>`, a browser's chrome – no server state); the current and the picked shots are
// shared by every step (storyboard-shots.ts). Pure: the storage comes in (`KeyValue`), wrapped so a blocked or full
// storage never stands in the way.

/** the dock panels in their fixed places, from the main view outwards (`order`); `rem` = the normal width */
export const PANELS = [
	{ kind: 'shot', icon: 'sliders', rem: 24 },
	{ kind: 'script', icon: 'file-text', rem: 20 },
	{ kind: 'rhythm', icon: 'activity', rem: 20 },
	{ kind: 'generate', icon: 'zap', rem: 22 }
] as const;
export type PanelKind = (typeof PANELS)[number]['kind'];

/** the Shot inspector's tabs */
export const SHOT_TABS = ['shot', 'camera', 'light', 'figures'] as const;
export type ShotTab = (typeof SHOT_TABS)[number];

export interface Step {
	id: string;
	/** the number key (1–6, fixed – a step that comes later keeps its number) */
	key: number;
	/** what it opens the first time */
	panels: PanelKind[];
	tab: ShotTab;
	/** false: its panels come with a later issue (Handheld, Export – #126); not shown */
	ready: boolean;
}

/** the six steps of the studio's design; Handheld and Export wait for #126 (their panels: the phone, takes, Output) */
export const STEPS: Step[] = [
	{ id: 'script', key: 1, panels: ['shot', 'script'], tab: 'shot', ready: true },
	{ id: 'shots', key: 2, panels: ['shot'], tab: 'shot', ready: true },
	{ id: 'layout', key: 3, panels: ['shot'], tab: 'camera', ready: true },
	{ id: 'handheld', key: 4, panels: [], tab: 'camera', ready: false },
	{ id: 'generate', key: 5, panels: ['generate'], tab: 'shot', ready: true },
	{ id: 'export', key: 6, panels: [], tab: 'shot', ready: false }
];
export const READY_STEPS = STEPS.filter((s) => s.ready);
export const DEFAULT_STEP = 'shots';

/** a step's remembered layout */
export interface StepLayout {
	panels: PanelKind[];
	tab: ShotTab;
}

/** a key-value storage (localStorage, a fake in the tests) */
export interface KeyValue {
	getItem(key: string): string | null;
	setItem(key: string, value: string): void;
}

const PREFIX = 'scutoons_creator:storyboard:step';
export const stepKey = (id: string) => `${PREFIX}:${id}`;

function read(storage: KeyValue | null, key: string): unknown {
	try {
		const raw = storage?.getItem(key);
		return raw == null ? null : JSON.parse(raw);
	} catch {
		return null;
	}
}
function write(storage: KeyValue | null, key: string, value: unknown) {
	try {
		storage?.setItem(key, JSON.stringify(value));
	} catch {
		// not remembered (private window, full storage)
	}
}

const isKind = (k: unknown): k is PanelKind => PANELS.some((p) => p.kind === k);
const isTab = (t: unknown): t is ShotTab => (SHOT_TABS as readonly unknown[]).includes(t);

/** a step's layout: the remembered one (unknown kinds dropped, each once) or the step's first */
export function loadLayout(storage: KeyValue | null, step: Step): StepLayout {
	const saved = read(storage, stepKey(step.id)) as Partial<StepLayout> | null;
	const panels = Array.isArray(saved?.panels) ? [...new Set(saved.panels.filter(isKind))] : [...step.panels];
	return { panels, tab: isTab(saved?.tab) ? saved.tab : step.tab };
}
export const saveLayout = (storage: KeyValue | null, id: string, layout: StepLayout) => write(storage, stepKey(id), layout);

/** the remembered current step (a step that isn't ready falls back) */
export function loadStep(storage: KeyValue | null): string {
	const id = read(storage, PREFIX);
	return READY_STEPS.some((s) => s.id === id) ? (id as string) : DEFAULT_STEP;
}
export const saveStep = (storage: KeyValue | null, id: string) => write(storage, PREFIX, id);

/** a panel opened or closed in a layout (the same layout when nothing changes) */
export function togglePanel(layout: StepLayout, kind: PanelKind, open: boolean): StepLayout {
	if (open === layout.panels.includes(kind)) return layout;
	return { ...layout, panels: open ? [...layout.panels, kind] : layout.panels.filter((k) => k !== kind) };
}

/** the step a number key picks (only ready steps), else null */
export const stepOfKey = (key: string): Step | null => READY_STEPS.find((s) => String(s.key) === key) ?? null;

/** this browser's storage, or null where it can't be reached */
export function browserStorage(): KeyValue | null {
	try {
		return typeof localStorage === 'undefined' ? null : localStorage;
	} catch {
		return null;
	}
}
