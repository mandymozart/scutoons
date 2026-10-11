import type { VNEvent } from '@scutoons/content';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { readerEnv } from './runner.ts';
import { runTrigger } from './script-runner.ts';

/** the group a slice belongs to (toon/group.svelte's context, visual-novel/group-scope.ts) */
export interface GroupScript {
	events: VNEvent[] | undefined;
	label: string | null;
	scope: string;
}

/**
 * Runs the events of a slice trigger or a placed item's tap (they share the box with the group's script) on the one VN
 * runner (`script-runner.ts` `runTrigger`): one run at a time, never while a question / choices are open, the script's
 * waiting message parked and always given back (box-state.ts). A `divert` here continues in ITS OWN group's script at
 * that named point (CUTOVER 10.27; linked groups – owner 2026-10-06). Returns the URL to navigate to (the caller
 * navigates), or null. While it runs (and waits for its turn) the page stays frozen.
 */
export class EventProcessor {
	constructor(
		private events: VNEvent[] | undefined,
		private game: GameContext,
		private vn: VisualNovelContext,
		private group?: GroupScript,
		/** where the events are (a slice trigger, a placed item): their texts' identity for visits (runner `origin`) */
		private origin = 'events',
		/** may an unlockLocation open the map? (no for intersect / hover: scrolling is never hijacked – MAP.md §3.2) */
		private mapReveal = true
	) {}

	run(): Promise<string | null> {
		const own = this.events ?? [];
		const label = this.group ? this.group.label : this.vn.scriptLabel;
		const script = this.group ? (this.group.events ?? []) : this.vn.script;
		console.log(`VN trigger: ${own.map((e) => e?.type).join(', ')}`);
		return runTrigger(own, {
			vn: this.vn,
			game: this.game,
			env: readerEnv(this.game),
			jumpProblem: (target) => this.vn.jumpProblem(target),
			script,
			label,
			scope: this.group?.scope ?? '',
			origin: this.origin,
			mapReveal: this.mapReveal
		});
	}
}
