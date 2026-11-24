import { describe, it } from 'node:test';
import { expect } from './expect.ts';
import { at, type PanelSpec, type Rect } from '../src/editor/floats.ts';
import {
	activate,
	activeOf,
	closeFree,
	defaultArrangement,
	dropIndex,
	dropTarget,
	freeId,
	freeSpec,
	hideTab,
	isEssential,
	isHidden,
	markerX,
	moveTab,
	notOffered,
	panelOf,
	readArrangement,
	showTab,
	tabsOf,
	tearOut,
	type Arrangement,
	type TabSpec
} from '../src/editor/panel-tabs.ts';

const PANELS: PanelSpec[] = [
	{ id: 'outline', name: 'Outline', icon: 'list', about: '', place: at('top-left', 0, 0), size: { w: 16, h: 'fill' }, min: { w: 12, h: 8 }, tabs: ['outline', 'history'] },
	{ id: 'inspector', name: 'Inspector', icon: 'sliders', about: '', place: at('top-right', 0, 0), size: { w: 24, h: 'fill' }, min: { w: 16, h: 10 }, tabs: ['inspector', 'problems'] },
	{ id: 'library', name: 'Library', icon: 'grid', about: '', place: at('bottom-left', 0, 0), size: { w: 'fill', h: 12 }, min: { w: 20, h: 8 }, tabs: ['library', 'wanted', 'info'] },
	{ id: 'tools', name: 'Tools', icon: 'tool', about: '', place: at('top-left', 17, 0), size: { w: 'content', h: 'content' }, min: { w: 2, h: 2 }, inline: true, essential: true }
];
const tab = (id: string, essential = false): TabSpec => ({ id, name: id[0].toUpperCase() + id.slice(1), icon: 'box', about: '', essential });
const TABS: TabSpec[] = [tab('outline'), tab('history'), tab('inspector', true), tab('problems'), tab('library'), tab('wanted'), tab('info')];
const start = () => defaultArrangement(PANELS);

describe('panel tabs: the default arrangement', () => {
	it('puts each declared tab in its panel, the first in front; a toolbar holds none', () => {
		const a = start();
		expect(a.panels).toEqual({ outline: ['outline', 'history'], inspector: ['inspector', 'problems'], library: ['library', 'wanted', 'info'] });
		expect(activeOf(a, 'library')).toBe('library');
		expect(a.panels.tools).toBeUndefined();
		expect(a.free).toEqual({});
	});

	it('a panel holding an essential tab is essential', () => {
		const a = start();
		expect(isEssential(a, 'inspector', TABS)).toBe(true);
		expect(isEssential(a, 'library', TABS)).toBe(false);
		const moved = moveTab(a, 'inspector', 'library', 0);
		expect(isEssential(moved, 'library', TABS)).toBe(true);
		expect(isEssential(moved, 'inspector', TABS)).toBe(false);
	});
});

describe('panel tabs: a tab into another panel', () => {
	it('lands before the tab at the index and comes to the front', () => {
		const a = moveTab(start(), 'history', 'library', 1);
		expect(a.panels.library).toEqual(['library', 'history', 'wanted', 'info']);
		expect(a.panels.outline).toEqual(['outline']);
		expect(activeOf(a, 'library')).toBe('history');
	});

	it('an index past the last appends it', () => {
		expect(moveTab(start(), 'history', 'inspector', 99).panels.inspector).toEqual(['inspector', 'problems', 'history']);
	});

	it('in its own panel it reorders: before or after the tab it is dropped on', () => {
		expect(moveTab(start(), 'library', 'library', 2).panels.library).toEqual(['wanted', 'library', 'info']);
		expect(moveTab(start(), 'library', 'library', 3).panels.library).toEqual(['wanted', 'info', 'library']);
		expect(moveTab(start(), 'info', 'library', 0).panels.library).toEqual(['info', 'library', 'wanted']);
		expect(moveTab(start(), 'wanted', 'library', 1).panels.library).toEqual(['library', 'wanted', 'info']);
	});

	it('the panel a front tab leaves shows its neighbour', () => {
		const a = activate(start(), 'library', 'wanted');
		expect(activeOf(moveTab(a, 'wanted', 'outline', 0), 'library')).toBe('info');
		const b = activate(start(), 'library', 'info');
		expect(activeOf(moveTab(b, 'info', 'outline', 0), 'library')).toBe('wanted');
	});

	it('a panel whose last tab leaves closes', () => {
		let a = moveTab(start(), 'outline', 'library', 0);
		a = moveTab(a, 'history', 'library', 0);
		expect(a.panels.outline).toBeUndefined();
		expect(panelOf(a, 'outline')).toBe('library');
		expect(a.panels.library.length).toBe(5);
	});

	it('an unknown tab or panel changes nothing', () => {
		const a = start();
		expect(moveTab(a, 'nothing', 'library', 0)).toBe(a);
		expect(moveTab(a, 'history', 'nowhere', 0)).toBe(a);
	});
});

describe('panel tabs: a tab out onto the stage', () => {
	it('becomes a free panel of its own, made from the tab', () => {
		const torn = tearOut(start(), 'wanted')!;
		expect(torn.panel).toBe('free:wanted');
		expect(torn.arrangement.panels['free:wanted']).toEqual(['wanted']);
		expect(torn.arrangement.free['free:wanted']).toBe('wanted');
		expect(torn.arrangement.panels.library).toEqual(['library', 'info']);
		expect(freeSpec(torn.panel, tab('wanted'))).toMatchObject({ id: 'free:wanted', name: 'Wanted', size: { w: 20, h: 22 }, min: { w: 12, h: 8 } });
	});

	it("the only tab of a panel is not torn out – that panel moves instead", () => {
		const a = moveTab(start(), 'history', 'library', 0);
		expect(tearOut(a, 'outline')).toBeNull();
	});

	it('a free panel that empties goes; a new one by the same tab is numbered while the first is still there', () => {
		let a = tearOut(start(), 'wanted')!.arrangement;
		a = moveTab(a, 'info', 'free:wanted', 1);
		a = moveTab(a, 'wanted', 'library', 0);
		expect(a.panels['free:wanted']).toEqual(['info']);
		expect(freeId(a, 'wanted')).toBe('free:wanted-2');
		a = moveTab(a, 'info', 'library', 0);
		expect(a.panels['free:wanted']).toBeUndefined();
		expect(a.free['free:wanted']).toBeUndefined();
	});

	it('an essential tab torn out makes its free panel essential', () => {
		const a = tearOut(start(), 'inspector')!;
		expect(isEssential(a.arrangement, a.panel, TABS)).toBe(true);
	});
});

describe('panel tabs: no tab is lost', () => {
	it('a closed free panel gives its tabs back to their home panels', () => {
		let a = tearOut(start(), 'wanted')!.arrangement;
		a = moveTab(a, 'history', 'free:wanted', 1);
		a = closeFree(a, 'free:wanted', PANELS);
		expect(a.panels['free:wanted']).toBeUndefined();
		expect(a.panels.library).toEqual(['library', 'info', 'wanted']);
		expect(a.panels.outline).toEqual(['outline', 'history']);
	});

	it('a home panel that had emptied comes back for them', () => {
		let a = moveTab(start(), 'outline', 'library', 0);
		a = moveTab(a, 'history', 'library', 0);
		a = tearOut(a, 'history')!.arrangement;
		expect(a.panels.outline).toBeUndefined();
		a = closeFree(a, 'free:history', PANELS);
		expect(a.panels.outline).toEqual(['history']);
	});

	it('closing a declared panel leaves its tabs where they are (the floats hide it)', () => {
		const a = start();
		expect(closeFree(a, 'library', PANELS)).toBe(a);
	});
});

describe('panel tabs: the arrangement in this browser', () => {
	it('reads back what it wrote', () => {
		let a = tearOut(start(), 'wanted')!.arrangement;
		a = moveTab(a, 'history', 'inspector', 1);
		expect(readArrangement(JSON.parse(JSON.stringify(a)), PANELS, TABS)).toEqual(a);
	});

	it('nothing or junk stored: the default', () => {
		expect(readArrangement(undefined, PANELS, TABS)).toEqual(start());
		expect(readArrangement('x', PANELS, TABS)).toEqual(start());
		expect(readArrangement({ panels: [] }, PANELS, TABS)).toEqual(start());
	});

	it('drops unknown tabs and panels and a tab stored twice; a tab missing goes home', () => {
		const stored = {
			panels: { library: ['library', 'gone', 'library'], inspector: ['inspector', 'problems', 'info'], nowhere: ['wanted'], 'free:x': ['outline'] },
			active: { library: 'gone', inspector: 'problems' },
			free: { 'free:x': 'nope' }
		};
		const a: Arrangement = readArrangement(stored, PANELS, TABS);
		expect(a.panels).toEqual({ library: ['library', 'wanted'], inspector: ['inspector', 'problems', 'info'], outline: ['outline', 'history'] });
		expect(a.active.library).toBe('library');
		expect(a.active.inspector).toBe('problems');
		expect(a.free).toEqual({});
	});

	it('a tab stored under an old id (an alias) comes back as the tab', () => {
		const panels: PanelSpec[] = PANELS.map((p) => (p.id === 'inspector' ? { ...p, tabs: ['inspector', 'log'] } : p));
		const tabs: TabSpec[] = [...TABS.filter((t) => t.id !== 'problems'), { ...tab('log'), aliases: ['problems'] }];
		const stored = { panels: { library: ['library', 'problems'], inspector: ['inspector'] }, active: { library: 'problems' }, free: {} };
		const a = readArrangement(stored, panels, tabs);
		expect(a.panels.library).toEqual(['library', 'log', 'wanted', 'info']);
		expect(a.active.library).toBe('log');
		expect(a.panels.inspector).toEqual(['inspector']);
	});
});

describe('panel tabs: where a dragged tab lands', () => {
	const strip: Rect = { x: 100, y: 10, w: 300, h: 30 };
	const tabs: Rect[] = [
		{ x: 100, y: 10, w: 60, h: 30 },
		{ x: 162, y: 10, w: 80, h: 30 },
		{ x: 244, y: 10, w: 50, h: 30 }
	];

	it('before the first tab whose middle lies right of the pointer', () => {
		expect(dropIndex(tabs, 90)).toBe(0);
		expect(dropIndex(tabs, 129)).toBe(0);
		expect(dropIndex(tabs, 131)).toBe(1);
		expect(dropIndex(tabs, 250)).toBe(2);
		expect(dropIndex(tabs, 280)).toBe(3);
	});

	it('the marker between two tabs, before the first, after the last', () => {
		expect(markerX(tabs, strip, 0)).toBe(100);
		expect(markerX(tabs, strip, 1)).toBe(161);
		expect(markerX(tabs, strip, 3)).toBe(294);
		expect(markerX([], strip, 0)).toBe(100);
	});

	it('the front-most panel under the pointer; over its strip by the tabs, elsewhere at its end; none = the stage', () => {
		const back = { id: 'back', rect: { x: 0, y: 0, w: 500, h: 500 }, strip: { x: 0, y: 0, w: 500, h: 30 }, tabs: [] };
		const front = { id: 'front', rect: { x: 90, y: 0, w: 320, h: 200 }, strip, tabs };
		expect(dropTarget(131, 20, [front, back])).toEqual({ panel: 'front', index: 1 });
		expect(dropTarget(131, 150, [front, back])).toEqual({ panel: 'front', index: 3 });
		expect(dropTarget(450, 20, [front, back])).toEqual({ panel: 'back', index: 0 });
		expect(dropTarget(700, 20, [front, back])).toBeNull();
	});
});

describe('panel tabs: a hidden tab (the Views eye)', () => {
	it('leaves its panel shown, keeps its place, and the front tab moves to a shown neighbour', () => {
		const a = hideTab(start(), 'library');
		expect(isHidden(a, 'library')).toBe(true);
		expect(tabsOf(a, 'library')).toEqual(['wanted', 'info']);
		expect(a.panels.library).toEqual(['library', 'wanted', 'info']);
		expect(activeOf(a, 'library')).toBe('wanted');
	});

	it('a panel with every tab hidden shows none; shown again a tab comes back in its place, in front', () => {
		const a = hideTab(hideTab(start(), 'outline'), 'history');
		expect(tabsOf(a, 'outline')).toEqual([]);
		const b = showTab(a, 'history');
		expect(tabsOf(b, 'outline')).toEqual(['history']);
		expect(activeOf(b, 'outline')).toBe('history');
		const c = showTab(b, 'outline');
		expect(tabsOf(c, 'outline')).toEqual(['outline', 'history']);
		expect(c.hidden).toBeUndefined();
	});

	it('a drop index counts the shown tabs only', () => {
		const a = moveTab(hideTab(start(), 'wanted'), 'history', 'library', 1);
		expect(a.panels.library).toEqual(['library', 'wanted', 'history', 'info']);
		expect(tabsOf(a, 'library')).toEqual(['library', 'history', 'info']);
	});

	it('is remembered; a stored hidden tab that is no longer known is dropped', () => {
		const a = hideTab(start(), 'problems');
		const read = readArrangement(JSON.parse(JSON.stringify({ ...a, hidden: ['problems', 'gone'] })), PANELS, TABS);
		expect(read.hidden).toEqual(['problems']);
		expect(tabsOf(read, 'inspector')).toEqual(['inspector']);
	});
});

describe('panel tabs: a tab whose content is gone', () => {
	it('a kept layout names it, but no panel offers it now: it is listed as not offered, its place kept', () => {
		const kept = activate(start(), 'library', 'wanted');
		const now = PANELS.map((p) => (p.id === 'library' ? { ...p, tabs: ['library', 'info'] } : p));
		expect(notOffered(kept, now)).toEqual(['wanted']);
		expect(notOffered(kept, PANELS)).toEqual([]);
		expect(kept.panels.library).toEqual(['library', 'wanted', 'info']);
		expect(activeOf({ ...kept, hidden: notOffered(kept, now) }, 'library')).toBe('library');
	});

	it('panels that list no tabs offer nothing to compare: none is left out', () => {
		expect(notOffered(start(), [PANELS[3]])).toEqual([]);
	});
});
