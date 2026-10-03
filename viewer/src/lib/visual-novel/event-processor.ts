import { labelIndex, type VNEvent } from '@scutoons/content';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { runScript } from './runner.ts';

/**
 * Runs the events of a slice trigger or a placed item's tap (they share the box with the group's script) on the one VN
 * runner (runner.ts). A `divert` here continues in the page's group script at that named point (CUTOVER 10.27: slice
 * triggers start the story at a point). Returns the URL to navigate to (the caller navigates), or null.
 */
export class EventProcessor {
	constructor(
		private events: VNEvent[] | undefined,
		private game: GameContext,
		private vn: VisualNovelContext
	) {}

	async run(): Promise<string | null> {
		const own = this.events ?? [];
		// one divert: the group's script from that point; anything else: these events
		const divert = own.length === 1 && own[0]?.type === 'divert' ? own[0].params?.to : null;
		const script = divert ? this.vn.script : own;
		const start = divert ? labelIndex(script, divert) : 0;
		if (divert && start < 0) {
			console.warn(`VN: no point "${divert}" in the group's script`);
			return null;
		}
		let url: string | null = null;
		// park the group's waiting message, bring it back when done (a click fires every event of the trigger one after
		// another, so the check runs after the click's task)
		this.vn.interrupt();
		try {
			const result = await runScript(script, start, {
				vn: this.vn,
				game: this.game,
				navigate: (to) => (url = to),
				currentLabel: () => this.vn.scriptLabel
			});
			if (result === 'done') this.vn.endOfEvents();
		} finally {
			setTimeout(() => this.vn.resumeIfIdle(), 0);
		}
		return url;
	}
}
