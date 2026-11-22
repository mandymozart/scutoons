// The creator's one panel / tab model (research/redesign/panels.html): every section is a tab (declared once in panels.ts); a
// panel is a set of tabs; a workspace is only a preset – which panels, their tabs, their places, which tab is main.
// A page fills the tabs it has content for (`useTabs`, shell.svelte.ts); the Toon, the Library, Settings and the Log are
// the shell's own. A tab with content that no preset panel holds still opens: it gets a closed panel of its own (Views).
// Pure: test/shell.test.ts; the tabs' names and icons: panels.ts.
import type { PanelSpec, TabSpec } from '@scutoons/ui/editor';
import type { WorkspaceKind } from './address.ts';

/** a default place (floats.ts `at`, written out: this module stays pure) */
const at = (corner: PanelSpec['place']['corner'], dx: number, dy: number, more: { collapsed?: boolean; closed?: boolean } = {}): PanelSpec['place'] => ({ corner, dx, dy, ...more });

export type TabId =
	| 'tree'
	| 'toon'
	| 'inspector'
	| 'kanon'
	| 'moods'
	| 'vocabulary'
	| 'flow'
	/** #763: the front item's weave of the spore network */
	| 'spores'
	| 'preview'
	| 'log'
	/** the Log's old id: stored layouts and presets naming it open the Log */
	| 'problems'
	| 'timeline'
	| 'script'
	| 'rhythm'
	| 'generate'
	| 'handheld'
	| 'output'
	| 'experience'
	| 'scenes'
	| 'library'
	| 'settings';

/** the tabs the shell draws itself (the others come from the front page) */
export const SHELL_TABS: readonly TabId[] = ['toon', 'library', 'settings', 'log', 'spores'];

/** old tab ids and the tab they name now */
export const TAB_ALIASES: Readonly<Record<string, TabId>> = { problems: 'log' };
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
	/**
	 * the tab on the stage when the workspace opens. None: the workspace's own view is the stage – it fills the whole
	 * window under the bar (a full-bleed stage such as an XR experience or a character stage needs no tab for it)
	 */
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
const LOG: PresetPanel = { id: 'log', tabs: ['log'], place: at('bottom-right', 0, 0, { closed: true }), size: { w: 32, h: 24 }, min: { w: 20, h: 8 } };

/** every workspace's preset (research/redesign/PLAN.md §4) */
export const PRESETS: Record<WorkspaceKind | 'home', Preset> = {
	home: { panels: [library({})] },
	toon: { panels: [left(['toon', 'tree']), right(['flow', 'log'], FOLDED), library()] },
	chapter: { panels: [left(['toon', 'tree']), right(['inspector', 'log', 'flow', 'spores']), preview(FOLDED), library()] },
	storyboard: { panels: [left(['toon', 'tree']), right(['inspector', 'script', 'rhythm', 'generate', 'handheld', 'output', 'log']), foot(['timeline']), library()] },
	character: { panels: [left(['tree']), right(['inspector', 'kanon', 'log']), foot(['moods', 'vocabulary']), library()] },
	catalog: { panels: [left(['tree']), right(['log'], FOLDED), library()] },
	map: { panels: [left(['tree']), right(['log'], FOLDED), library()] },
	scene: { panels: [left(['tree']), library()] },
	// the XR editor (#482): the stage is the experience, the whole window; the panels float over it
	experience: { panels: [left(['tree']), right(['inspector', 'experience', 'spores']), foot(['scenes']), library()], clear: false },
	flow: { panels: [right(['log'], FOLDED)] },
	sound: { panels: [right(['log'], FOLDED), library()] },
	text: { panels: [left(['tree']), right(['log'], FOLDED)] },
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

/** the tabs on a workspace's stage: those the person put there (empty: none), else the preset's `main` */
export function stageOf(workspace: string, view: { stage?: readonly string[] }): readonly string[] {
	if (view.stage) return view.stage.map(tabIdOf);
	const main = PRESETS[workspace as WorkspaceKind | 'home']?.main;
	return main ? [main] : [];
}

/** the tabs with content: the shell's own (the Toon only with a toon) and those the front page fills */
export function availableTabs(filled: readonly string[], toon: string | null): Set<string> {
	return new Set([...SHELL_TABS.filter((t) => t !== 'toon' || !!toon), ...filled]);
}
