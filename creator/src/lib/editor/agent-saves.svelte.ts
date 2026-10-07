// The Ask panel's saves, for the open editors (owner 2026-10-04: a draft the agent saved conflicted with the next save in
// the editor): the panel reports every document the agent saved (`report`), the page's DocSession reloads that file
// when it has no unsaved changes, or shows the conflict bar right away (Reload / Overwrite) when it has.
import { shared } from '$lib/utils/shared.ts';
import { draftSaves } from '$lib/editor/draft-saves.svelte.ts';

export interface AgentSave {
	/** stored key, e.g. `chapter:catpope:1:en` */
	key: string;
	version: number;
	/** increases with every report (the same key twice is two reports) */
	seq: number;
}

class AgentSaves {
	last = $state<AgentSave | null>(null);
	private seq = 0;

	report(key: string, version: number) {
		this.last = { key, version, seq: ++this.seq };
		// the dock's Reader reloads too (BACKLOG 10.49); the toon is the key's second part (`chapter:<toon>:…`)
		draftSaves.report(key.split(':')[1] ?? '');
	}
}

export const agentSaves = shared('agent-saves', () => new AgentSaves());
