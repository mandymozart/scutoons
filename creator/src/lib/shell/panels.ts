// The creator's panels, declared once (research/redesign/PLAN.md §3): every workspace shows the ones whose content is
// there and opens its helpful ones alongside; the rest wait closed in the main menu's list. Until the areas move onto
// the stage (#665 …), a workspace's background is the area's page and the page lends the Tree (its own list) and the
// Document (Publish · Problems · History).
import { at, type PanelSpec, type TabSpec } from '@scutoons/ui/editor';
import { m } from '$lib/paraglide/messages.js';
import type { WorkspaceKind } from './address.ts';

export type PanelId = 'tree' | 'toon' | 'document' | 'library' | 'settings';

/** what the front page has to show (a panel without content is not offered) */
export interface Available {
	tree: boolean;
	doc: boolean;
}

/** the helpful panels each workspace opens alongside (the others start closed) */
const OPEN: Record<WorkspaceKind | 'home', readonly PanelId[]> = {
	home: [],
	toon: ['toon', 'tree', 'document'],
	chapter: ['tree', 'document'],
	storyboard: ['tree', 'document'],
	character: ['tree', 'document'],
	catalog: ['tree', 'document'],
	map: ['tree', 'document'],
	scene: ['tree'],
	experience: ['tree'],
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
		{ id: 'library', name: m['shell.library'](), icon: 'grid', about: m['shell.library_about'](), size: { w: 28, h: 'fill' } },
		{ id: 'settings', name: m['shell.settings'](), icon: 'settings', about: m['shell.settings_about'](), size: { w: 26, h: 'fill' } }
	];
}

/** the panels of a workspace given what the front page has; `open` = its helpful ones, open by default */
export function panelSpecs(workspace: WorkspaceKind | 'home', available: Available): PanelSpec[] {
	const open = OPEN[workspace];
	const closed = (id: PanelId) => !open.includes(id);
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
		{ id: 'library', name: m['shell.library'](), icon: 'grid', about: '', place: at('top-right', 0, 0, { closed: closed('library') }), size: { w: 28, h: 'fill' }, min: { w: 18, h: 12 }, tabs: ['library'] },
		{ id: 'settings', name: m['shell.settings'](), icon: 'settings', about: '', place: at('top-right', 0, 0, { closed: true }), size: { w: 26, h: 'fill' }, min: { w: 20, h: 12 }, tabs: ['settings'] }
	];
	return all.filter((p) => (p.id === 'tree' ? available.tree : p.id === 'document' ? available.doc : p.id === 'toon' ? workspace === 'toon' : true));
}
