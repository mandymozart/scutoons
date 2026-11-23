// The creator's tabs, declared once (research/redesign/panels.html): every section is a tab; the presets (which
// panels, their tabs, places, which tab is main) are presets.ts.
import type { TabSpec } from '@scutoons/ui/editor';
import { m } from '$lib/paraglide/messages.js';

export * from './presets.ts';

export function tabSpecs(): TabSpec[] {
	return [
		{ id: 'toon', name: m['shell.panel_toon'](), icon: 'book', about: m['shell.panel_toon_about']() },
		{ id: 'tree', name: m['shell.panel_tree'](), icon: 'list', about: m['shell.panel_tree_about']() },
		{ id: 'inspector', name: m['creator.inspector'](), icon: 'sliders', about: m['shell.panel_inspector_about']() },
		{ id: 'kanon', name: m['creator.kanon'](), icon: 'file-text', about: m['shell.panel_kanon_about']() },
		{ id: 'expressions', name: m['character.expressions'](), icon: 'smile', about: m['character.expressions_about']() },
		{ id: 'rig', name: m['character.rig'](), icon: 'box', about: m['character.rig_about']() },
		{ id: 'voice', name: m['character.voice'](), icon: 'mic', about: m['character.voice_about']() },
		{ id: 'cast', name: m['character.cast'](), icon: 'users', about: m['character.cast_about']() },
		{ id: 'generations', name: m['character.generations'](), icon: 'zap', about: m['character.generations_about']() },
		{ id: 'flow', name: m['creator.flow'](), icon: 'git-merge', about: m['shell.panel_flow_about']() },
		{ id: 'preview', name: m['shell.panel_preview'](), icon: 'smartphone', about: m['shell.panel_preview_about']() },
		{ id: 'log', name: m['shell.panel_log'](), icon: 'list', about: m['shell.panel_log_about'](), aliases: ['problems'] },
		{ id: 'history', name: m['shell.history_menu'](), icon: 'clock', about: m['shell.history_about']() },
		{ id: 'timeline', name: m['storyboard.tl_title'](), icon: 'film', about: m['shell.panel_timeline_about']() },
		{ id: 'script', name: m['storyboard.panel_script'](), icon: 'file-text', about: m['storyboard.panel_script']() },
		{ id: 'rhythm', name: m['storyboard.panel_rhythm'](), icon: 'activity', about: m['storyboard.panel_rhythm']() },
		{ id: 'generate', name: m['storyboard.panel_generate'](), icon: 'zap', about: m['storyboard.panel_generate']() },
		{ id: 'handheld', name: m['storyboard.panel_handheld'](), icon: 'smartphone', about: m['storyboard.panel_handheld']() },
		{ id: 'output', name: m['storyboard.panel_output'](), icon: 'download', about: m['storyboard.panel_output']() },
		{ id: 'library', name: m['shell.library'](), icon: 'grid', about: m['shell.library_about']() },
		{ id: 'settings', name: m['shell.settings'](), icon: 'settings', about: m['shell.settings_about']() }
	];
}
