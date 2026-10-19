// The studio's steps (owner 2026-10-19, research/STUDIO-LAYOUT.md concept C stage 1 = B: "Script · Shots · Layout ·
// Handheld · Generate · Export", "it makes sense for it to always look the same"): one layout – the shot list, the main
// view, the panels in their fixed places, the timeline along the bottom – and per step only which panels are open, the
// Shot panel's tab, the main view and the timeline's fold differ. Each step remembers its own (localStorage
// `storyboard-studio:step:<id>`, a browser's chrome – no server state); the panels' widths are per step too
// (`storyboard-studio:<step>:<kind>`, App). The selected shot is shared by all steps (studio.svelte.js).

/** the dock panels in their fixed places, from the main view outwards (`order`) */
export const PANELS = [
	{ kind: 'shot', title: 'Shot', icon: 'sliders', rem: 24 },
	{ kind: 'script', title: 'Script', icon: 'file-text', rem: 20 },
	{ kind: 'handheld', title: 'Handheld', icon: 'smartphone', rem: 22 },
	{ kind: 'generate', title: 'Generate', icon: 'zap', rem: 22 },
	{ kind: 'prompt', title: 'Luma', icon: 'send', rem: 24 },
	{ kind: 'variants', title: 'Variants', icon: 'layers', rem: 18 },
	{ kind: 'output', title: 'Output', icon: 'download', rem: 24 }
];

/** the Shot panel's tabs (stage 2 turns them into the inspector's tab column) */
export const SHOT_TABS = [
	{ value: 'shot', label: 'Shot', icon: 'image', title: 'What it shows' },
	{ value: 'camera', label: 'Camera', icon: 'video', title: 'Format + camera' },
	{ value: 'light', label: 'Light', icon: 'sun', title: 'Light' },
	{ value: 'figures', label: 'Figures', icon: 'users', title: 'Figures' }
];

/** the steps and what each opens the first time (each fits 1440: shots + a 24rem main + its panels) */
export const STEPS = [
	{ id: 'script', label: 'Script', title: 'The scene\'s script next to the shot – what each shot shows', panels: ['shot', 'script'], tab: 'shot', timeline: false },
	{ id: 'shots', label: 'Shots', title: 'The shots in order – what each shows, their rhythm on the timeline', panels: ['shot'], tab: 'shot', timeline: true },
	{ id: 'layout', label: 'Layout', title: 'Blocking: format, camera, light, figures in the 3D view', panels: ['shot'], tab: 'camera', timeline: true },
	{ id: 'handheld', label: 'Handheld', title: 'Film the shot with the phone, trim the take on the timeline', panels: ['handheld'], tab: 'camera', timeline: true },
	{ id: 'generate', label: 'Generate', title: 'Generate the shot, pick and approve the result', panels: ['generate', 'variants'], tab: 'shot', timeline: true },
	{ id: 'export', label: 'Export', title: 'The chapter\'s outputs: handheld videos, downloads, the draft', panels: ['output'], tab: 'shot', timeline: false }
];

const KEY = 'storyboard-studio:step';
const read = (k) => { try { return JSON.parse(localStorage.getItem(k) ?? 'null'); } catch { return null; } };
const write = (k, v) => { try { localStorage.setItem(k, JSON.stringify(v)); } catch { /* not remembered */ } };

/** a step's remembered layout: { panels: kind[], tab, view, timeline } – `view` is the main view ('3d'; stage 2 adds
 * Frames / Compare) */
function load(step) {
	const saved = read(`${KEY}:${step.id}`);
	const kinds = PANELS.map((p) => p.kind);
	return {
		panels: Array.isArray(saved?.panels) ? saved.panels.filter((k) => kinds.includes(k)) : [...step.panels],
		tab: SHOT_TABS.some((t) => t.value === saved?.tab) ? saved.tab : step.tab,
		view: '3d',
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
const keep = () => write(`${KEY}:${steps.current}`, here());

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
export function setTimeline(open) {
	if (here().timeline === open) return;
	here().timeline = open;
	keep();
}
