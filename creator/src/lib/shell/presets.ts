// The creator's one panel / tab model (research/redesign/panels.html): every section is a tab (declared once in panels.ts); a
// panel is a set of tabs; a workspace is only a preset – the opened item's main view on the stage, which panels, their tabs,
// their places. A workspace without a main view shows its page's own view under the stage (presetStart).
// A page fills the tabs it has content for (`useTabs`, shell.svelte.ts); the Toon, the Library, Settings, the Log and History are
// the shell's own. A tab with content that no preset panel holds still opens: it gets a closed panel of its own (Views).
// Pure: test/shell.test.ts; the tabs' names and icons: panels.ts.
import type { PanelSpec, TabSpec } from '@scutoons/ui/editor';
import type { WorkspaceKind } from './address.ts';

/** a default place (floats.ts `at`, written out: this module stays pure) */
const at = (corner: PanelSpec['place']['corner'], dx: number, dy: number, more: { collapsed?: boolean; closed?: boolean } = {}): PanelSpec['place'] => ({ corner, dx, dy, ...more });

export type TabId =
	| 'tree'
	/** the open item's parts in order, each a frame (lap 2 C3): a storyboard's shots, a chapter's slices, an experience's scenes */
	| 'filmstrip'
	/** the character's views: its sheet (one picture), its 3D place – any of them, and Expressions, can take the stage */
	| 'sheet'
	| '3d'
	/** the Generate workspace: the spore canvas, the spores to weave in, a generator's runs */
	| 'canvas'
	| 'spores'
	| 'runs'
	| 'toon'
	| 'inspector'
	| 'kanon'
	/** a writing page's sections and its facts (lap 2 C14) */
	| 'outline'
	| 'facts'
	/** a character's sections beside the Kanon: its moods' pictures, its model, its voice, where it is cast, how its pictures are made */
	| 'expressions'
	| 'rig'
	| 'voice'
	| 'cast'
	| 'generations'
	| 'flow'
	/** #763: the front item's weave of the spore network (its own id: 'spores' is the Generate workspace's list) */
	| 'weave'
	| 'preview'
	| 'log'
	/** what is under the pointer (deck-kit info.rs): opened by the member, in no preset */
	| 'info'
	/** the open item's saved versions (was the bar's History menu) */
	| 'history'
	/** the Log's old id: stored layouts and presets naming it open the Log */
	| 'problems'
	/** the character's old Moods panel tabs: they open Expressions */
	| 'moods'
	| 'vocabulary'
	| 'timeline'
	| 'script'
	| 'rhythm'
	| 'generate'
	| 'handheld'
	| 'output'
	| 'experience'
	| 'scenes'
	/** Sound design: the module's code (its main view), its parameters, its wiring as nodes, the rig's assignment */
	| 'rust'
	| 'parameters'
	| 'nodes'
	| 'assignment'
	/** Sound design: the deck playing the rig (the being), its outputs' meters, its knobs, the compile's problems and console */
	| 'monitor'
	| 'meters'
	| 'knobs'
	| 'diagnostics'
	| 'console'
	| 'library'
	| 'settings';

/** the tabs the shell draws itself (the others come from the front page) */
export const SHELL_TABS: readonly TabId[] = ['toon', 'library', 'settings', 'log', 'history', 'info', 'weave'];

/** old tab ids and the tab they name now */
export const TAB_ALIASES: Readonly<Record<string, TabId>> = { problems: 'log', moods: 'expressions', vocabulary: 'expressions' };
/** a tab id with its alias resolved */
export const tabIdOf = (id: string): string => TAB_ALIASES[id] ?? id;

/** one panel of a preset: its tabs (the first in front), place and size; `folded` / `closed` by default */
export interface PresetPanel {
	id: string;
	tabs: readonly TabId[];
	place: PanelSpec['place'];
	size: PanelSpec['size'];
	min: PanelSpec['min'];
}

export interface Preset {
	panels: readonly PresetPanel[];
	/** the opened item's main view: on the stage when the workspace starts (it must sit in one of the panels) */
	main?: TabId;
	/** the view's default for clear of the bar (a full-bleed workspace: false) */
	clear?: boolean;
}

const left = (tabs: readonly TabId[], more: { collapsed?: boolean; closed?: boolean } = {}): PresetPanel => ({ id: 'left', tabs, place: at('top-left', 0, 0, more), size: { w: 18, h: 'fill' }, min: { w: 12, h: 8 } });
const right = (tabs: readonly TabId[], more: { collapsed?: boolean; closed?: boolean } = {}, h: number | 'fill' = 'fill'): PresetPanel => ({ id: 'right', tabs, place: at('top-right', 0, 0, more), size: { w: 25, h }, min: { w: 18, h: 8 } });
const foot = (tabs: readonly TabId[], more: { collapsed?: boolean; closed?: boolean } = {}): PresetPanel => ({ id: 'foot', tabs, place: at('bottom-left', 0, 0, more), size: { w: 'fill', h: 11 }, min: { w: 20, h: 6 } });
const preview = (more: { collapsed?: boolean; closed?: boolean } = {}): PresetPanel => ({ id: 'preview', tabs: ['preview'], place: at('bottom-right', 0, 0, more), size: { w: 24, h: 26 }, min: { w: 16, h: 12 } });
const library = (more: { collapsed?: boolean; closed?: boolean } = { collapsed: true }): PresetPanel => ({ id: 'library', tabs: ['library'], place: at('bottom-right', 0, 0, more), size: { w: 28, h: 'fill' }, min: { w: 18, h: 12 } });
const FOLDED = { collapsed: true };
/** the views that are not on the stage, folded at the bottom left */
const views = (tabs: readonly TabId[]): PresetPanel => ({ id: 'views', tabs, place: at('bottom-left', 0, 0, FOLDED), size: { w: 30, h: 24 }, min: { w: 16, h: 12 } });
const LOG: PresetPanel = { id: 'log', tabs: ['log'], place: at('bottom-right', 0, 0, { closed: true }), size: { w: 32, h: 24 }, min: { w: 20, h: 8 } };

/** every workspace's preset (research/redesign/PLAN.md §4) */
export const PRESETS: Record<WorkspaceKind | 'home', Preset> = {
	home: { panels: [library({})] },
	toon: { panels: [left(['toon', 'tree']), right(['flow', 'log'], FOLDED), preview(FOLDED), library()] },
	chapter: { panels: [left(['toon', 'tree', 'filmstrip']), right(['inspector', 'log', 'flow', 'weave']), preview(FOLDED), library()] },
	storyboard: { panels: [left(['filmstrip']), right(['inspector', 'script', 'rhythm', 'generate', 'handheld', 'output', 'log']), foot(['timeline']), library()] },
	character: { main: 'sheet', panels: [{ ...right(['kanon', 'expressions', 'rig', 'voice', 'cast', 'generations']), size: { w: 34, h: 'fill' } }, views(['sheet', '3d']), library()] },
	catalog: { panels: [left(['tree']), right(['log'], FOLDED), library()] },
	map: { panels: [left(['tree']), right(['log'], FOLDED), library()] },
	// lap 2 C5: the scene's still is the whole back, its facts and versions a panel over it; no list of its own (the Library opens scenes)
	scene: { panels: [right(['facts'], {}, 30), library()], clear: false },
	// the XR editor (#482): the stage is the experience, the whole window; the panels float over it
	experience: { panels: [left(['tree']), right(['inspector', 'experience', 'weave']), foot(['scenes']), library()], clear: false },
	flow: { panels: [right(['log'], FOLDED)] },
	// the module editor on the host (lap 2 T10): the code on the stage; the monitor at the right with its meters and knobs
	// under it; the compile's Problems and Console at the bottom left; Parameters · Nodes · Assignment folded beside the Log
	sound: {
		main: 'rust',
		panels: [
			right(['log'], FOLDED),
			{ id: 'module', tabs: ['rust', 'parameters', 'nodes', 'assignment'], place: at('top-right', 11, 0, FOLDED), size: { w: 40, h: 'fill' }, min: { w: 20, h: 12 } },
			{ id: 'monitor', tabs: ['monitor'], place: at('top-right', 0, 3.5), size: { w: 26, h: 18 }, min: { w: 14, h: 10 } },
			{ id: 'knobs', tabs: ['knobs'], place: at('bottom-right', 0, 4.5), size: { w: 26, h: 11 }, min: { w: 12, h: 7 } },
			{ id: 'meters', tabs: ['meters'], place: at('bottom-right', 27, 4.5), size: { w: 9, h: 11 }, min: { w: 6, h: 7 } },
			{ id: 'diagnostics', tabs: ['diagnostics'], place: at('bottom-left', 0, 0), size: { w: 28, h: 10 }, min: { w: 14, h: 6 } },
			{ id: 'console', tabs: ['console'], place: at('bottom-left', 29, 0), size: { w: 28, h: 10 }, min: { w: 14, h: 6 } },
			library()
		]
	},
	// the writing page (lap 2 C14): the text on the stage, the Outline and the Facts (with the Versions) folded at rest
	text: { panels: [left(['outline', 'tree'], FOLDED), { ...right(['facts', 'history', 'log'], FOLDED, 20), size: { w: 22, h: 20 } }] },
	generate: { main: 'canvas', panels: [left(['spores', 'canvas']), right(['runs', 'log'], FOLDED), library()] },
	help: { panels: [] }
};

const SETTINGS: PresetPanel = { id: 'settings', tabs: ['settings'], place: at('top-right', 0, 0, { closed: true }), size: { w: 26, h: 'fill' }, min: { w: 20, h: 12 } };

/**
 * The panels of a workspace for the tabs that have content now (`available`): the preset's panels with those of their
 * tabs (an old tab id read as its tab), the Library, the Log and Settings everywhere (closed unless the preset opens them), and a closed panel of its own for a
 * tab no panel holds – so every tab with content can be shown (Views).
 */
export function panelSpecs(workspace: WorkspaceKind | 'home', available: ReadonlySet<string>, specs: readonly TabSpec[]): PanelSpec[] {
	const preset = PRESETS[workspace];
	const panels: PresetPanel[] = preset.panels.map((p) => ({ ...p, tabs: [...new Set(p.tabs.map(tabIdOf) as TabId[])] }));
	if (!panels.some((p) => p.id === 'library')) panels.push(library({ closed: true }));
	if (!panels.some((p) => p.tabs.includes('log'))) panels.push(LOG);
	panels.push(SETTINGS);
	const placed = new Set(panels.flatMap((p) => p.tabs));
	for (const tab of specs) if (available.has(tab.id) && !placed.has(tab.id as TabId)) panels.push({ id: tab.id, tabs: [tab.id as TabId], place: at('top-right', 0, 0, { closed: true }), size: { w: 24, h: 24 }, min: { w: 16, h: 8 } });
	const name = (id: string) => specs.find((t) => t.id === id);
	return panels.flatMap((p) => {
		const tabs = p.tabs.filter((t) => available.has(t));
		if (!tabs.length) return [];
		const first = name(tabs[0]);
		return [{ id: p.id, name: first?.name ?? p.id, icon: first?.icon ?? 'square', about: first?.about ?? '', place: p.place, size: p.size, min: p.min, tabs }];
	});
}

/** where a workspace starts without a saved layout: its main view on the stage (null: the specs' places, the page under the stage) */
export function presetStart(workspace: WorkspaceKind | 'home'): { stage: { layout: 'one'; slots: [string] } } | null {
	const main = PRESETS[workspace]?.main;
	return main ? { stage: { layout: 'one', slots: [main] } } : null;
}

/** the tabs with content: the shell's own (the Toon only with a toon) and those the front page fills */
export function availableTabs(filled: readonly string[], toon: string | null): Set<string> {
	return new Set([...SHELL_TABS.filter((t) => t !== 'toon' || !!toon), ...filled]);
}
