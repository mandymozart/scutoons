// The creator's panels, declared once (research/redesign/PLAN.md §3): every workspace shows the ones whose content is
// there and opens its helpful ones alongside; the rest wait closed in the main menu's list. Until the areas move onto
// the stage (#665 …), a workspace's background is the area's page and the page lends the Tree (its own list), the
// Document (Publish · Problems · History) and the tabs it draws itself (`shell.lent`: the Inspector, the Moods …).
import { at, type PanelSpec, type TabSpec } from '@scutoons/ui/editor';
import { m } from '$lib/paraglide/messages.js';
import type { WorkspaceKind } from './address.ts';

export type PanelId = 'tree' | 'toon' | 'document' | 'inspector' | 'moods' | 'library' | 'settings';

/** what the front page has to show (a panel without content is not offered) */
export interface Available {
	tree: boolean;
	doc: boolean;
	/** the tab ids the page draws itself (`shell.lent`) */
	lent?: readonly string[];
}

/** the helpful panels each workspace opens alongside (the others start closed) */
const OPEN: Record<WorkspaceKind | 'home', readonly PanelId[]> = {
	home: [],
	toon: ['toon', 'tree', 'document'],
	chapter: ['tree', 'document'],
	storyboard: ['tree', 'document'],
	character: ['tree', 'inspector', 'moods'],
	catalog: ['tree', 'document'],
	map: ['tree', 'document'],
	scene: ['tree'],
	flow: ['document'],
	sound: ['document'],
	text: ['tree', 'document'],
	help: []
};

export function tabSpecs(): TabSpec[] {
	return [
		{ id: 'tree', name: m['shell.panel_tree'](), icon: 'list', about: m['shell.panel_tree_about']() },
		{ id: 'toon', name: m['shell.panel_toon'](), icon: 'book', about: m['shell.panel_toon_about']() },
		{ id: 'publish', name: m['shell.panel_publish'](), icon: 'upload-cloud', about: m['shell.panel_publish_about']() },
		{ id: 'problems', name: m['creator.problems'](), icon: 'alert-triangle', about: m['shell.panel_problems_about']() },
		{ id: 'history', name: m['creator.history'](), icon: 'clock', about: m['shell.panel_history_about']() },
		{ id: 'inspector', name: m['creator.inspector'](), icon: 'sliders', about: m['shell.panel_inspector_about']() },
		{ id: 'kanon', name: m['creator.kanon'](), icon: 'file-text', about: m['shell.panel_kanon_about']() },
		{ id: 'moods', name: m['creator.characters_moods'](), icon: 'smile', about: m['shell.panel_moods_about']() },
		{ id: 'vocabulary', name: m['shell.panel_vocabulary'](), icon: 'book-open', about: m['shell.panel_vocabulary_about']() },
		{ id: 'library', name: m['shell.library'](), icon: 'grid', about: m['shell.library_about'](), size: { w: 28, h: 'fill' } },
		{ id: 'settings', name: m['shell.settings'](), icon: 'settings', about: m['shell.settings_about'](), size: { w: 26, h: 'fill' } }
	];
}

/** the panels of a workspace given what the front page has; `open` = its helpful ones, open by default */
export function panelSpecs(workspace: WorkspaceKind | 'home', available: Available): PanelSpec[] {
	const open = OPEN[workspace];
	const closed = (id: PanelId) => !open.includes(id);
	const lentTabs = (tabs: string[]) => tabs.filter((t) => available.lent?.includes(t));
	const all: (PanelSpec & { id: PanelId })[] = [
		{ id: 'tree', name: m['shell.panel_tree'](), icon: 'list', about: '', place: at('top-left', 0, 0, { closed: closed('tree') }), size: { w: 18, h: 'fill' }, min: { w: 12, h: 10 }, tabs: ['tree'] },
		{ id: 'toon', name: m['shell.panel_toon'](), icon: 'book', about: '', place: at('top-left', 0, 0, { closed: closed('toon') }), size: { w: 15, h: 15 }, min: { w: 12, h: 6 }, tabs: ['toon'] },
		{
			id: 'document',
			name: m['shell.panel_publish'](),
			icon: 'upload-cloud',
			about: '',
			place: at('top-right', 0, 0, { closed: closed('document') }),
			size: { w: 22, h: 17 },
			min: { w: 16, h: 6 },
			tabs: ['publish', 'problems', 'history']
		},
		{ id: 'inspector', name: m['creator.inspector'](), icon: 'sliders', about: '', place: at('top-right', 0, 0, { closed: closed('inspector') }), size: { w: 24, h: 'fill' }, min: { w: 18, h: 12 }, essential: true, tabs: lentTabs(['inspector', 'kanon']) },
		{ id: 'library', name: m['shell.library'](), icon: 'grid', about: '', place: at('top-right', 0, 0, { closed: closed('library') }), size: { w: 28, h: 'fill' }, min: { w: 18, h: 12 }, tabs: ['library'] },
		{ id: 'moods', name: m['creator.characters_moods'](), icon: 'smile', about: '', place: at('bottom-left', 0, 0, { closed: closed('moods') }), size: { w: 'fill', h: 11 }, min: { w: 20, h: 6 }, tabs: lentTabs(['moods', 'vocabulary']) },
		{ id: 'settings', name: m['shell.settings'](), icon: 'settings', about: '', place: at('top-right', 0, 0, { closed: true }), size: { w: 26, h: 'fill' }, min: { w: 20, h: 12 }, tabs: ['settings'] }
	];
	return all.filter((p) => (p.id === 'tree' ? available.tree : p.id === 'document' ? available.doc : p.id === 'toon' ? workspace === 'toon' : p.tabs ? p.tabs.length > 0 : true));
}
