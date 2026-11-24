// What the launcher lists, read from the lists that already exist (places, the core's Items to open, the workspace row,
// the front workspace's views, the Library's own search, the shell's commands) and what a pick does. The ranking is
// launcher.ts (pure).
import { goto } from '$app/navigation';
import { languageCommand } from '@scutoons/ui';
import type { AssetFamily, LibraryKind } from '@scutoons/media';
import { fileAddress, library } from '$lib/editor/library.svelte.ts';
import { shelf } from '$lib/editor/library-shelf.svelte.ts';
import { kindWord } from '$lib/labels.ts';
import type { CreatorNav } from '$lib/nav.svelte.ts';
import { m } from '$lib/paraglide/messages.js';
import { getLocale, locales, setLocale } from '$lib/paraglide/runtime.js';
import { toast } from '$lib/toasts.ts';
import { hrefOf, iconOf, resolveTyped } from './address.ts';
import type { LaunchEntry } from './launcher.ts';
import { openings } from './openings.svelte.ts';
import { tabSpecs } from './panels.ts';
import { places, type Place } from './places.svelte.ts';
import { shell } from './shell.svelte.ts';
import { workspaceName } from './workspace-names.ts';
import { workspaceRow } from './workspaces.ts';
import { withOwn } from './layouts.ts';
import { layouts } from './layouts.svelte.ts';
import { canMakeOwn, newWorkspace, resetLabel, saveWorkspace } from './workspace-actions.ts';

/** what the lists need loaded once the launcher opens */
export function loadSources(nav: CreatorNav) {
	places.load(nav.toons);
	void openings.load();
	void library.load();
}

const item = (p: Place): LaunchEntry => ({ id: `item:${p.address}`, kind: 'item', label: p.label, hint: p.hint ?? kindWord(p.address) ?? undefined, type: kindWord(p.address) ?? undefined, address: p.address, keywords: p.keywords, icon: p.icon, href: p.href });

/** every item: the toons with their seasons and chapters (the front toon first), the areas, every kind the core opens */
export function items(nav: CreatorNav, toon: string | null): LaunchEntry[] {
	return [...places.toons(nav.toons, (id) => nav.toonName(id), toon), ...places.areas(), ...places.experiences(), ...Object.values(openings.rows).flat()].map(item);
}

/** an address typed by hand that leads to a page, though no list has it (yet) */
export function typed(query: string): LaunchEntry[] {
	const href = resolveTyped(query);
	const address = query.trim();
	return href ? [{ id: `item:${address}`, kind: 'item', label: address, hint: m['shell.launch_address'](), type: kindWord(address) ?? undefined, address, icon: iconOf(address), href }] : [];
}

export function workspaces(): LaunchEntry[] {
	return withOwn(workspaceRow(shell.use, shell.state.tabs), layouts.state.own).map((e) => ({ id: `workspace:${e.id}`, kind: 'workspace', label: e.name ?? workspaceName(e.kind), hint: m['shell.launch_workspace'](), icon: 'layout', href: e.href }));
}

/** the front workspace's views (Settings is a command) */
export function views(): LaunchEntry[] {
	return tabSpecs(shell.kind)
		.filter((t) => t.id !== 'settings' && shell.available.has(t.id))
		.map((t) => ({ id: `view:${t.id}`, kind: 'view', label: t.name, hint: m['shell.launch_view'](), keywords: t.about, icon: t.icon }));
}

const capital = (word: string) => word.charAt(0).toUpperCase() + word.slice(1);
const ASSET_ICONS: Record<LibraryKind, string> = { image: 'image', video: 'film', audio: 'music', model: 'box', animation: 'activity', file: 'file' };
const MAX_ASSETS = 20;

/** the Library's own search over every file (the core's view, read only), for what was typed */
export function assets(query: string): LaunchEntry[] {
	const core = library.core;
	if (!core || !query.trim()) return [];
	const view = core.view({ scope: 'all', grouping: 'none', sort: 'name', query: query.trim(), kind: null, special: null, origin: null, fit: null, single: false, now: Date.now() });
	const families = new Map(core.families(false).map((f) => [f.key, f]));
	return view.order
		.slice(0, MAX_ASSETS)
		.map((key) => families.get(key))
		.filter((f): f is AssetFamily => !!f)
		.map((f) => ({ id: `asset:${f.key}`, kind: 'asset', label: f.title || f.name, hint: f.folder, type: capital(m[`library.kind_${f.kind}`]()), keywords: `${f.name} ${f.folder} ${f.tags.join(' ')} ${f.kind}`, icon: ASSET_ICONS[f.kind] ?? 'file' }));
}

interface Command {
	id: string;
	label: string;
	icon: string;
	keywords?: string;
	run: () => void;
}

function commands(logout: () => unknown): Command[] {
	const day = shell.settings.theme === 'day';
	const language = languageCommand({ locales, current: getLocale(), setLocale, label: (locale) => m['palette.language']({ locale: locale.toUpperCase() }) });
	return [
		// a press of the bar's Publish (its one dialog, RULES H18)
		{ id: 'publish', label: m['shell.launch_publish'](), icon: 'upload-cloud', keywords: 'publish live veröffentlichen', run: () => document.querySelector<HTMLElement>('[data-testid="bar-publish"]')?.click() },
		{ id: 'settings', label: m['shell.settings'](), icon: 'settings', keywords: 'settings preferences einstellungen', run: () => shell.open('settings') },
		{ id: 'views', label: m['shell.views'](), icon: 'eye', keywords: 'views panels ansichten', run: () => shell.openViews() },
		{ id: 'dashboard', label: m['shell.dashboard'](), icon: 'menu', keywords: 'switcher dashboard home workspaces', run: () => shell.toggleSwitcher(true) },
		{ id: 'docs', label: m['shell.docs'](), icon: 'book-open', keywords: 'docs help documentation doku', run: () => void window.open('/docs', '_blank', 'noopener') },
		{ id: 'theme', label: day ? m['shell.launch_night']() : m['shell.launch_day'](), icon: day ? 'moon' : 'sun', keywords: 'theme day night dark light tag nacht', run: () => shell.change({ theme: day ? 'night' : 'day' }) },
		{ id: 'save-workspace', label: m['shell.launch_save'](), icon: 'save', keywords: 'save workspace layout speichern arbeitsbereich', run: () => void saveWorkspace() },
		{ id: 'reset', label: `${m['shell.launch_reset']()} – ${resetLabel()}`, icon: 'rotate-ccw', keywords: 'reset workspace layout zurücksetzen last save', run: () => shell.resetView() },
		...(canMakeOwn() ? [{ id: 'new-workspace', label: m['shell.ws_new'](), icon: 'copy', keywords: 'new workspace save as layout neuer arbeitsbereich', run: () => void newWorkspace() }] : []),
		{ id: language.id, label: language.label, icon: 'globe', keywords: language.keywords, run: language.run },
		{ id: 'logout', label: m['creator.sign_out'](), icon: 'log-out', keywords: 'logout log out sign out abmelden', run: () => void logout() }
	];
}

export function commandEntries(logout: () => unknown): LaunchEntry[] {
	return commands(logout).map((c) => ({ id: `command:${c.id}`, kind: 'command', label: c.label, hint: m['shell.launch_command'](), keywords: c.keywords, icon: c.icon }));
}

/** a pick: an item through the core's open rule when it is one of its kinds, a workspace, a view, a file, a command */
export function launch(entry: LaunchEntry, logout: () => unknown) {
	const [kind, ...rest] = entry.id.split(':');
	const id = rest.join(':');
	switch (kind) {
		case 'item': {
			const listed = Object.values(openings.rows).some((rows) => rows.some((p) => p.address === id));
			if (listed) {
				const answer = openings.open(id);
				if (!answer.ok) return void toast(answer.why, { type: 'warning' });
				const href = hrefOf(answer.address);
				if (href) void goto(href);
				return;
			}
			if (entry.href) void goto(entry.href);
			return;
		}
		case 'workspace': {
			const own = layouts.state.own.find((o) => o.id === id);
			layouts.show(own?.kind ?? id, id);
			if (entry.href) void goto(entry.href);
			return;
		}
		case 'view': {
			const floats = shell.floats;
			if (floats && floats.stateOf(id) === 'hidden') floats.setState(id, 'panel');
			shell.open(id);
			return;
		}
		case 'asset': {
			shell.open('library');
			shelf.setPlace('all', library.address);
			shelf.query = entry.label;
			const family = library.core?.families(false).find((f) => `asset:${f.key}` === id);
			if (family) library.click(fileAddress(family));
			return;
		}
		case 'command':
			commands(logout)
				.find((c) => c.id === id)
				?.run();
	}
}
