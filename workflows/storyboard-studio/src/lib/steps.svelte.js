// The studio's steps (owner 2026-10-19, research/STUDIO-LAYOUT.md concept C stage 1 = B: "Script · Shots · Layout ·
// Handheld · Generate · Export", "it makes sense for it to always look the same"): one layout – the shot list, the main
// view, the panels in their fixed places, the timeline along the bottom – and per step only which panels are open, the
// Shot panel's tab, the main view (the timeline's view mode) and the timeline's fold differ. Each step remembers its own
// (localStorage `storyboard-studio:step:v2:<id>` – v2: the view modes and the timeline open only in Handheld + Generate,
// owner 2026-10-19; a browser's chrome – no server state); the panels' widths are per step too
// (`storyboard-studio:<step>:<kind>`, App). The current and the picked shots are shared by all steps (studio.svelte.js).

/** the dock panels in their fixed places, from the main view outwards (`order`) */
export const PANELS = [
	{ kind: 'shot', title: 'Shot', icon: 'sliders', rem: 24 },
	{ kind: 'script', title: 'Script', icon: 'file-text', rem: 20 },
	{ kind: 'handheld', title: 'Handheld', icon: 'smartphone', rem: 22 },
	{ kind: 'generate', title: 'Generate', icon: 'zap', rem: 22 },
	{ kind: 'prompt', title: 'Luma', icon: 'send', rem: 24 },
	{ kind: 'variants', title: 'Variants', icon: 'grid', rem: 18 },
	{ kind: 'output', title: 'Output', icon: 'download', rem: 24 }
];

/** the Shot panel's tabs (stage 2 turns them into the inspector's tab column) – words only: with icons "Figures" ran
 * into its frame at the panel's 24rem */
export const SHOT_TABS = [
	{ value: 'shot', label: 'Shot', title: 'What it shows' },
	{ value: 'camera', label: 'Camera', title: 'Format + camera' },
	{ value: 'light', label: 'Light', title: 'Light' },
	{ value: 'figures', label: 'Figures', title: 'Figures' }
];

/** the timeline's view modes – which clip each picked shot is and what the main view shows (owner 2026-10-19: "like in
 * blender, rendered view port, vs. shaded, vs. wireframe vs box. in our case, it is shot/take/rendered/generated";
 * timeline-lanes.js: the fallbacks) */
export const VIEW_MODES = [
	{ value: 'shot', label: 'Shot', title: 'Every shot a still of 5 s – the 3D view' },
	{ value: 'take', label: 'Take', title: 'The handheld take – its camera in the 3D view, its in / out on the timeline (else a still)' },
	{ value: 'rendered', label: 'Rendered', title: 'The handheld render of the take (beauty pass) – else the take, a still' },
	{ value: 'generated', label: 'Generated', title: 'The generated video (the approved one, else the newest) – else rendered, the take, a still' }
];

/** the steps and what each opens the first time (each fits 1440: shots + a 24rem main + its panels); the timeline open
 * only where time matters – Handheld + Generate (owner 2026-10-19) */
export const STEPS = [
	{ id: 'script', label: 'Script', title: 'The scene\'s script next to the shot – what each shot shows', panels: ['shot', 'script'], tab: 'shot', view: 'shot', timeline: false },
	{ id: 'shots', label: 'Shots', title: 'The shots in order – what each shows', panels: ['shot'], tab: 'shot', view: 'shot', timeline: false },
	{ id: 'layout', label: 'Layout', title: 'Blocking: format, camera, light, figures in the 3D view', panels: ['shot'], tab: 'camera', view: 'shot', timeline: false },
	{ id: 'handheld', label: 'Handheld', title: 'Film the shot with the phone, trim the take on the timeline', panels: ['handheld'], tab: 'camera', view: 'take', timeline: true },
	{ id: 'generate', label: 'Generate', title: 'Generate the shot, pick and approve the result', panels: ['generate', 'variants'], tab: 'shot', view: 'generated', timeline: true },
	{ id: 'export', label: 'Export', title: 'The chapter\'s outputs: handheld videos, downloads, the draft', panels: ['output'], tab: 'shot', view: 'rendered', timeline: false }
];

const KEY = 'storyboard-studio:step';
/** a step's layout (v2: the view modes, the new timeline defaults – the stage-1 layouts are left behind) */
const layoutKey = (id) => `${KEY}:v2:${id}`;
const read = (k) => { try { return JSON.parse(localStorage.getItem(k) ?? 'null'); } catch { return null; } };
const write = (k, v) => { try { localStorage.setItem(k, JSON.stringify(v)); } catch { /* not remembered */ } };

/** a step's remembered layout: { panels: kind[], tab, view, timeline } – `view` = the main view: the timeline's view mode
 * (VIEW_MODES) */
function load(step) {
	const saved = read(layoutKey(step.id));
	const kinds = PANELS.map((p) => p.kind);
	return {
		panels: Array.isArray(saved?.panels) ? saved.panels.filter((k) => kinds.includes(k)) : [...step.panels],
		tab: SHOT_TABS.some((t) => t.value === saved?.tab) ? saved.tab : step.tab,
		view: VIEW_MODES.some((m) => m.value === saved?.view) ? saved.view : step.view,
		timeline: typeof saved?.timeline === 'boolean' ? saved.timeline : step.timeline
	};
}

const first = read(KEY);
export const steps = $state({
	/** the current step's id */
	current: STEPS.some((s) => s.id === first) ? first : 'layout',
	/** every step's layout */
	of: Object.fromEntries(STEPS.map((s) => [s.id, load(s)]))
});

/** the current step's layout */
export const here = () => steps.of[steps.current];
const keep = () => write(layoutKey(steps.current), here());

export function setStep(id) {
	if (!STEPS.some((s) => s.id === id) || id === steps.current) return;
	steps.current = id;
	write(KEY, id);
}

export const isOpen = (kind) => here().panels.includes(kind);
export function setOpen(kind, open) {
	const layout = here();
	if (open === layout.panels.includes(kind)) return;
	layout.panels = open ? [...layout.panels, kind] : layout.panels.filter((k) => k !== kind);
	keep();
}
export function setTab(tab) {
	here().tab = tab;
	keep();
}
export function setView(view) {
	if (here().view === view || !VIEW_MODES.some((m) => m.value === view)) return;
	here().view = view;
	keep();
}
export function setTimeline(open) {
	if (here().timeline === open) return;
	here().timeline = open;
	keep();
}
