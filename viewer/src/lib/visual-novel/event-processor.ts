import { labelIndex, type VNEvent } from '@scutoons/content';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { runScript } from './runner.ts';
import { readerKnown } from '$lib/reader-name.svelte.ts';

/** the group a slice belongs to (toon/group.svelte's context, visual-novel/group-scope.ts) */
export interface GroupScript {
	events: VNEvent[] | undefined;
	label: string | null;
	scope: string;
}

/**
 * Runs the events of a slice trigger or a placed item's tap (they share the box with the group's script) on the one VN
 * runner (runner.ts). A `divert` here continues in ITS OWN group's script at that named point (CUTOVER 10.27: slice
 * triggers start the story at a point; linked groups – several scripts on a page – owner 2026-10-06). Returns the URL to
 * navigate to (the caller navigates), or null. While it runs the page stays frozen (`vn.beginRun`).
 */
export class EventProcessor {
	constructor(
		private events: VNEvent[] | undefined,
		private game: GameContext,
		private vn: VisualNovelContext,
		private group?: GroupScript,
		/** where the events are (a slice trigger, a placed item): their texts' identity for visits (runner `origin`) */
		private origin = 'events'
	) {}

	async run(): Promise<string | null> {
		this.vn.beginRun();
		try {
			return await this.runEvents();
		} finally {
			this.vn.endRun();
		}
	}

	private async runEvents(): Promise<string | null> {
		const own = this.events ?? [];
		// one divert: the group's script from that point; anything else: these events
		const divert = own.length === 1 && own[0]?.type === 'divert' ? own[0].params?.to : null;
		const label = this.group ? this.group.label : this.vn.scriptLabel;
		const script = divert ? (this.group ? (this.group.events ?? []) : this.vn.script) : own;
		console.log(`VN trigger: ${own.map((e) => e?.type).join(', ')}${divert ? ` → point "${divert}" (${script.length} events in the script)` : ''}`);
		const start = divert ? labelIndex(script, divert) : 0;
		if (divert && start < 0) {
			console.warn(`VN: no point "${divert}" in the group's script`);
			return null;
		}
		// a question is open (another trigger's or the script's choices): this trigger waits until it is answered, so two
		// questions never merge (owner 2026-10-03: "Take it" showed next to the stroll end's choices)
		while (this.vn.state.choicesVisible || this.vn.state.pendingChoices.length) await new Promise((r) => setTimeout(r, 100));
		let url: string | null = null;
		// who reads is known first (runner.ts waits for it too): a group script that started earlier and waits for the
		// session shows its message before this run parks it – else its message would come up under the trigger's
		await readerKnown();
		// park the group's waiting message, bring it back when done (a click fires every event of the trigger one after
		// another, so the check runs after the click's task)
		this.vn.interrupt();
		try {
			const result = await runScript(script, start, {
				vn: this.vn,
				game: this.game,
				navigate: (to) => (url = to),
				currentLabel: () => label,
				scope: this.group?.scope ?? '',
				origin: divert ? 'script' : this.origin
			});
			if (result === 'done') this.vn.endOfEvents();
		} finally {
			setTimeout(() => this.vn.resumeIfIdle(), 0);
		}
		return url;
	}
}
