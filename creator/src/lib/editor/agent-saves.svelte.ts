// The Ask panel's saves, for the open editors (owner 2026-10-04: a draft the agent saved conflicted with the next save in
// the editor): the panel reports every document the agent saved (`report`), the page's DocSession reloads that file
// when it has no unsaved changes, or shows the conflict bar right away (Reload / Overwrite) when it has.

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
	}
}

export const agentSaves = new AgentSaves();
