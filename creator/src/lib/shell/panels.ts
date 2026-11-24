// The creator's tabs, declared once (research/redesign/panels.html): every section is a tab; the presets (which
// panels, their tabs, places, which tab is main) are presets.ts.
import type { TabSpec } from '@scutoons/ui/editor';
import { m } from '$lib/paraglide/messages.js';
import type { WorkspaceKind } from './address.ts';

export * from './presets.ts';

/** the Filmstrip's name where it shows one kind of part (lap 2 C3: one panel for shots, slices and XR scenes) */
const FILMSTRIP_NAMES: Partial<Record<string, () => string>> = {
	storyboard: () => m['shell.filmstrip_shots'](),
	chapter: () => m['shell.filmstrip_slices'](),
	experience: () => m['shell.filmstrip_scenes']()
};

/** the tabs, named for the workspace `kind` where a tab's name depends on it (the Filmstrip) */
export function tabSpecs(kind?: WorkspaceKind | 'home' | string): TabSpec[] {
	return [
		{
			id: 'filmstrip',
			name: (kind ? FILMSTRIP_NAMES[kind]?.() : undefined) ?? m['shell.panel_filmstrip'](),
			icon: 'film',
			about: m['shell.panel_filmstrip_about'](),
			// the Storyboard's shots were its Tree: a layout kept from then shows them here
			aliases: kind === 'storyboard' ? ['tree'] : undefined
		},
		{ id: 'toon', name: m['shell.panel_toon'](), icon: 'book', about: m['shell.panel_toon_about']() },
		{ id: 'tree', name: m['shell.panel_tree'](), icon: 'list', about: m['shell.panel_tree_about']() },
		{ id: 'inspector', name: m['creator.inspector'](), icon: 'sliders', about: m['shell.panel_inspector_about']() },
		{ id: 'sheet', name: m['character.view_sheet'](), icon: 'user', about: m['character.view_sheet_about']() },
		{ id: '3d', name: m['character.view_3d'](), icon: 'box', about: m['character.view_3d_about']() },
		{ id: 'canvas', name: m['shell.gen_canvas'](), icon: 'share-2', about: m['shell.gen_canvas_about']() },
		{ id: 'spores', name: m['shell.gen_spores'](), icon: 'plus-circle', about: m['shell.gen_spores_about']() },
		{ id: 'runs', name: m['shell.gen_runs'](), icon: 'play', about: m['shell.gen_runs_about']() },
		{ id: 'kanon', name: m['creator.kanon'](), icon: 'file-text', about: m['shell.panel_kanon_about']() },
		{ id: 'outline', name: m['shell.panel_outline'](), icon: 'list', about: m['shell.panel_outline_about']() },
		{ id: 'facts', name: m['shell.panel_facts'](), icon: 'info', about: m['shell.panel_facts_about']() },
		{ id: 'expressions', name: m['character.expressions'](), icon: 'smile', about: m['character.expressions_about']() },
		{ id: 'rig', name: m['character.rig'](), icon: 'box', about: m['character.rig_about']() },
		{ id: 'voice', name: m['character.voice'](), icon: 'mic', about: m['character.voice_about']() },
		{ id: 'cast', name: m['character.cast'](), icon: 'users', about: m['character.cast_about']() },
		{ id: 'generations', name: m['character.generations'](), icon: 'zap', about: m['character.generations_about']() },
		{ id: 'flow', name: m['creator.flow'](), icon: 'git-merge', about: m['shell.panel_flow_about']() },
		{ id: 'weave', name: m['shell.panel_spores'](), icon: 'share-2', about: m['shell.panel_spores_about']() },
		{ id: 'preview', name: m['shell.panel_preview'](), icon: 'smartphone', about: m['shell.panel_preview_about']() },
		{ id: 'log', name: m['shell.panel_log'](), icon: 'list', about: m['shell.panel_log_about'](), aliases: ['problems'] },
		{ id: 'info', name: m['shell.panel_info'](), icon: 'info', about: m['shell.panel_info_about']() },
		{ id: 'history', name: m['shell.history_menu'](), icon: 'clock', about: m['shell.history_about']() },
		{ id: 'timeline', name: m['storyboard.tl_title'](), icon: 'film', about: m['shell.panel_timeline_about']() },
		{ id: 'script', name: m['storyboard.panel_script'](), icon: 'file-text', about: m['storyboard.panel_script']() },
		{ id: 'rhythm', name: m['storyboard.panel_rhythm'](), icon: 'activity', about: m['storyboard.panel_rhythm']() },
		{ id: 'generate', name: m['storyboard.panel_generate'](), icon: 'zap', about: m['storyboard.panel_generate']() },
		{ id: 'handheld', name: m['storyboard.panel_handheld'](), icon: 'smartphone', about: m['storyboard.panel_handheld']() },
		{ id: 'output', name: m['storyboard.panel_output'](), icon: 'download', about: m['storyboard.panel_output']() },
		{ id: 'experience', name: m['shell.panel_experience'](), icon: 'aperture', about: m['shell.panel_experience_about']() },
		{ id: 'scenes', name: m['shell.panel_scenes'](), icon: 'film', about: m['shell.panel_scenes_about']() },
		{ id: 'library', name: m['shell.library'](), icon: 'grid', about: m['shell.library_about']() },
		{ id: 'settings', name: m['shell.settings'](), icon: 'settings', about: m['shell.settings_about']() }
	];
}
