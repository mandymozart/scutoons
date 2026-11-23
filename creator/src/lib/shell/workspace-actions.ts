// The front workspace's layout commands – Views' bottom lines and the launcher's commands call these (crafts-v2.html view
// `save`): Save (the arrangement now becomes the saved state, with the account), Back to the last save, Back to the craft
// default / the preset (the save taken back), Save as new workspace (named; it joins the bar's row and the dashboard),
// and Delete for an own workspace. Moving things stays live and is kept per browser between saves (the floats).
import { askDialog, confirmDialog } from '@scutoons/ui';
import { craftName } from '@scutoons/api';
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';
import { toast } from '$lib/toasts.ts';
import { ROW } from './workspaces.ts';
import { workspaceName } from './workspace-names.ts';
import { crafts } from './crafts.svelte.ts';
import { cleanName, ownId } from './layouts.ts';
import { layouts } from './layouts.svelte.ts';
import { shell } from './shell.svelte.ts';
import type { WorkspaceKind } from './address.ts';

const isKind = (kind: string): kind is WorkspaceKind => ROW.includes(kind as WorkspaceKind);

/** a workspace's name: an own one's, else the creator workspace's ('home' has none) */
export function nameOf(id: string): string {
	const own = layouts.state.own.find((o) => o.id === id);
	if (own) return own.name;
	return isKind(id) ? workspaceName(id) : m['shell.dashboard']();
}

/** the front workspace is an own one */
export const isOwn = (id = shell.workspace): boolean => layouts.state.own.some((o) => o.id === id);

/** an own workspace can be made from the front one (not Home, not Help) */
export const canMakeOwn = (): boolean => isKind(shell.kind);

function craftLabel(id: string): string {
	const craft = crafts.list.find((c) => c.id === id);
	return m['shell.ws_back_craft']({ craft: craft ? craftName(craft, getLocale()) : id });
}

/** what Reset goes back to, in words: the last save, else the craft default, else the preset */
export function resetLabel(id = shell.workspace): string {
	const start = layouts.startKind(id);
	if (start.kind === 'saved') return m['shell.ws_back_saved']();
	return start.kind === 'craft' ? craftLabel(start.craft) : m['shell.ws_back_preset']();
}

/** what taking the save back goes to, in words (the craft default, or the preset) – null when nothing is saved to take back */
export function defaultLabel(id = shell.workspace): string | null {
	if (isOwn(id) || !layouts.state.saved[id]) return null;
	const d = layouts.state.defaults[id];
	return d ? craftLabel(d.craft) : m['shell.ws_back_preset']();
}

const said = (where: 'account' | 'browser', name: string) =>
	where === 'account' ? toast(m['shell.ws_saved']({ name })) : toast(m['shell.ws_saved_browser']({ name }), { type: 'warning' });

/** the front workspace's arrangement becomes its saved state */
export async function saveWorkspace(): Promise<void> {
	const floats = shell.floats;
	if (!floats) return;
	const id = shell.workspace;
	said(await layouts.save(id, floats.layout), nameOf(id));
}

/** back to the last save (never saved: the craft default, else the preset) */
export function resetWorkspace(): void {
	shell.resetView();
}

/** the save taken back: the craft default, else the preset (a creator workspace only) */
export async function defaultWorkspace(): Promise<void> {
	const id = shell.workspace;
	if (isOwn(id)) return;
	await layouts.remove(id);
	shell.floats?.setStart(layouts.startOf(id));
	shell.resetView();
}

/** a new own workspace from the front one's arrangement, named – it becomes the front workspace */
export async function newWorkspace(): Promise<void> {
	const floats = shell.floats;
	const kind = shell.kind;
	if (!floats || !isKind(kind)) return;
	const asked = await askDialog({
		title: m['shell.ws_new_title'](),
		message: m['shell.ws_new_message']({ name: nameOf(shell.workspace) }),
		label: m['shell.ws_new_label'](),
		value: m['shell.ws_new_suggest']({ name: nameOf(shell.workspace) }),
		select: true,
		confirmLabel: m['shell.ws_new_confirm']()
	});
	const name = cleanName(asked ?? '');
	if (!name) return;
	const id = ownId([...layouts.state.own.map((o) => o.id), ...Object.keys(layouts.state.saved)]);
	const layout = floats.layout;
	const where = await layouts.save(id, layout, { name, kind });
	layouts.show(kind, id);
	said(where, name);
}

/** an own workspace gone (its kind shows the creator workspace again) */
export async function deleteWorkspace(): Promise<void> {
	const id = shell.workspace;
	if (!isOwn(id)) return;
	const name = nameOf(id);
	if (!(await confirmDialog({ title: m['shell.ws_delete_title']({ name }), message: m['shell.ws_delete_message'](), confirmLabel: m['shell.ws_delete_confirm']() }))) return;
	await layouts.remove(id);
	toast(m['shell.ws_deleted']({ name }));
}
