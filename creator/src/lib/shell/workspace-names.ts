import { m } from '$lib/paraglide/messages.js';
import type { WorkspaceKind } from './address.ts';

/** a workspace's name (the bar's row, the dashboard's cards) */
const NAMES: Record<WorkspaceKind, () => string> = {
	toon: () => m['shell.ws_toon'](),
	chapter: () => m['shell.ws_chapter'](),
	storyboard: () => m['shell.ws_storyboard'](),
	character: () => m['shell.ws_character'](),
	scene: () => m['shell.ws_scene'](),
	experience: () => m['shell.ws_experience'](),
	catalog: () => m['shell.ws_catalog'](),
	map: () => m['shell.ws_map'](),
	flow: () => m['shell.ws_flow'](),
	sound: () => m['shell.ws_sound'](),
	text: () => m['shell.ws_text'](),
	generate: () => m['shell.ws_generate'](),
	help: () => m['shell.help']()
};

export const workspaceName = (kind: WorkspaceKind): string => NAMES[kind]();

/** the deck's workspaces as the deck's row names them (server/schema/tables/deck-link.json `cards`); another: its word */
const DECK_NAMES: Record<string, () => string> = {
	stage: () => m['shell.deck_ws_stage'](),
	rigging: () => m['shell.deck_ws_rigging'](),
	show: () => m['shell.deck_ws_show']()
};

export const deckWorkspaceName = (workspace: string): string => DECK_NAMES[workspace]?.() ?? workspace;
