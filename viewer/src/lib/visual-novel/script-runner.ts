import { ASK_TEXT_LIMIT, ASK_TEXT_MAX_LENGTH, GIVE_ITEM_MAX, evaluateText, isStoryPath, labelIndex, testCondition, type ExprResolver, type ExprValue, type FxParams, type JumpToTarget, type VNEvent } from '@scutoons/content';
import { buildJumpToUrl } from './jump-to.ts';

/**
 * The one VN script runner's core (CUTOVER 10.27; pure – no Svelte: `runner.ts` wires it to the viewer, the node tests
 * drive it with the pure box of box-state.ts). Story flow: `label` = a named point, `divert` (event or a picked choice)
 * continues there – no page change; `jumpTo` is navigation only (`navigate`, then the run ends – a jump into a place
 * that isn't there stays: the run goes on, the follow-up bar comes); `if` skips an event; `setVariable` writes a
 * variable; `askText` waits for a typed answer (Skip always works). A question whose every choice is hidden asks
 * nothing (the run goes on). A run that keeps diverting without waiting for the reader stops (a loop guard).
 *
 * Waiting (owner 2026-10-06: no dead ends – box-state.ts): every await is for the reader (a click, a choice, an answer –
 * each answerable on screen), for a text box's effect (ends by itself, its tap layer continues), for the session (a
 * bounded check) or – a page script only (`yieldToRuns`) – for a slice-trigger run that holds the box to end.
 */

/** what the runner needs of the VN (the context implements it; the node tests a pure box + a stage stub) */
export interface RunnerVn {
	findInScene(id: string, persona?: string): unknown;
	joinScene(refs: any[]): void;
	leaveScene(refs: any[]): void;
	showMessage(character: any, text: string, wait: boolean): void;
	hideMessage(): void;
	waitForClick(): Promise<void>;
	waitForContinue(): Promise<void>;
	ask(q: { placeholder: string; maxLength: number; answer: string }): Promise<string | null>;
	addChoice(choice: { label: string; onSelect: any; from?: string | null }): void;
	showChoices(): boolean;
	whenChoicesClosed(): Promise<void>;
	dropUnshownChoices(count: number): void;
	isNavigationTriggered(): boolean;
	takePendingDivert(): string | null;
	showTextBox(params: any, scope?: string): boolean;
	setTextBoxResolver(resolver: () => void): void;
	whenNoRun(): Promise<void>;
	runHolds(): boolean;
	/** an impact (`fx`, agents/FX.md): plays it, returns how long it lasts (ms) – none: not drawn here (tests) */
	playFx?(params: FxParams, scope?: string): number;
}

/** what the runner needs of the game state */
export interface RunnerGame {
	getVariable(name: string): string | number | undefined;
	setVariable(name: string, value: string | number): void;
	readonly state: { readonly toonId: string };
}

/** the reader around the run (the viewer: the session, the reader's name, the expression resolver) */
export interface RunnerEnv {
	/** who reads is known (a member's name / access load with the session – bounded) */
	readerKnown(): Promise<void>;
	/** the reader's name (askText scope reader) */
	setReaderName(name: string): Promise<void>;
	/** the reader's name's longest form */
	readerNameMax: number;
	/** a text as the reader sees it (`at` = its identity: visits of `{a|b}` lists) – none: story variables only */
	evaluate?(source: string, at: string): string;
	/** expression conditions' names / functions */
	conditions?: ExprResolver;
	/** a choice's / a setVariable's actions (variables, item grants) */
	setVariable(action: { set: { variable: string; value: string | number } } | { add: { variable: string; value: number } }): void;
	/** give the reader a catalog item (`giveItem` – BACKLOG 10.38; the inventory's rules apply) */
	giveItem?(item: string, qty: number): void;
	/** take a catalog item from the reader (`takeItem` – BACKLOG 10.38): all or nothing, the story goes on either way */
	takeItem?(take: { item: string; qty?: number; repeat?: unknown }): void;
	/** plain text, trimmed, cut (askText answers) */
	cleanText(text: string, limit: number): string;
}

export interface CoreDeps {
	vn: RunnerVn;
	game: RunnerGame;
	env: RunnerEnv;
	/** go to another page (group / chapter); the run ends */
	navigate: (url: string) => void;
	/** why a jump can't go where it says (a group label missing in this chapter …) – null: it can */
	jumpProblem?: (target: JumpToTarget) => string | null;
	/** the page went away */
	cancelled?: () => boolean;
	/** the group the reader is on (a random jump never picks it) */
	currentLabel?: () => string | null;
	/** the group of the page whose script / slice events run (its text boxes) */
	scope?: string;
	/** where the events come from, for their texts' identity: `script` (default), a slice trigger, a placed item */
	origin?: string;
	/** a page script: pause before each event while a slice-trigger run holds the box (box-state.ts) */
	yieldToRuns?: boolean;
}

export type RunResult = 'done' | 'navigated' | 'cancelled';

/** diverts in a row without anything that waits for the reader before the run is stopped as a loop */
export const LOOP_LIMIT = 200;

export async function runEvents(events: readonly VNEvent[], start: number, deps: CoreDeps): Promise<RunResult> {
	const { vn, game, env } = deps;
	const cancelled = () => deps.cancelled?.() ?? false;
	const label = () => deps.currentLabel?.() ?? null;
	const get = (name: string) => game.getVariable(name);
	const show = (source: string, at: number): string =>
		env.evaluate
			? env.evaluate(source, `${label() ?? deps.scope ?? ''}/${deps.origin ?? 'script'}/${at}`)
			: evaluateText(source, { name: (path) => (isStoryPath(path) ? (get(path.join('.')) as ExprValue) : undefined) }).text;
	let i = Math.max(0, start);
	let jumpsWithoutWait = 0;
	/** choices this run added that no showChoices has shown yet (dropped when the run ends) */
	let unshown = 0;

	const divertTo = (to: string): boolean => {
		const at = labelIndex(events, to);
		if (at < 0) {
			console.warn(`VN: no point "${to}" in this script`);
			return false;
		}
		if (++jumpsWithoutWait > LOOP_LIMIT) throw new Error(`VN: diverts loop (last: "${to}")`);
		i = at;
		return true;
	};

	await env.readerKnown();
	try {
		while (i < events.length) {
			// a slice-trigger run holds the box: wait – checked again right before the step (a run may take it in between)
			if (deps.yieldToRuns) while (vn.runHolds()) await vn.whenNoRun();
			if (cancelled()) return 'cancelled';
			const event = events[i];
			i++;
			if (!event || !testCondition(event.if, get, env.conditions)) continue;

			switch (event.type) {
				case 'label':
					break;

				case 'divert':
					divertTo(event.params?.to);
					break;

				case 'setVariable': {
					const { variable, value, op } = event.params;
					if (op === 'add') env.setVariable({ add: { variable, value: Number(value) || 0 } });
					// a number stays a number (the clock adds to it); text stays text
					else env.setVariable({ set: { variable, value: typeof value === 'number' ? value : String(value) } });
					break;
				}

				case 'giveItem': {
					const { item, qty } = event.params ?? {};
					if (typeof item === 'string' && item) env.giveItem?.(item, Math.min(GIVE_ITEM_MAX, Math.max(1, Math.floor(Number(qty) || 1))));
					break;
				}

				case 'takeItem': {
					const take = event.params;
					if (take && typeof take.item === 'string' && take.item) env.takeItem?.(take);
					break;
				}

				case 'showMessage': {
					const { text, character, persona, wait = false } = event.params;
					let speaker = null;
					if (character) {
						speaker = vn.findInScene(character, persona) ?? null;
						if (!speaker) {
							console.error(`Character "${character}" is not in the scene. Add them with joinScene first.`);
							break;
						}
					}
					vn.showMessage(speaker, show(text, i - 1), false);
					if (wait) {
						jumpsWithoutWait = 0;
						await vn.waitForClick();
					}
					break;
				}

				case 'askText': {
					// the question in the box with one input line; the page waits for Enter / ✓ / the answer button (EXPRESSIONS.md §6)
					const { text, character, persona, placeholder, maxLength, answer, default: fallback, scope = 'toon', variable } = event.params;
					const speaker = character ? (vn.findInScene(character, persona) ?? null) : null;
					if (character && !speaker) console.error(`Character "${character}" is not in the scene. Add them with joinScene first.`);
					vn.showMessage(speaker, show(text, i - 1), false);
					jumpsWithoutWait = 0;
					const limit = Math.min(scope === 'reader' ? env.readerNameMax : ASK_TEXT_LIMIT, Math.max(1, Math.floor(Number(maxLength) || ASK_TEXT_MAX_LENGTH)));
					const typed = await vn.ask({ placeholder: placeholder ?? '', maxLength: limit, answer: (answer ?? '').trim() });
					if (cancelled()) return 'cancelled';
					// plain text, trimmed (never markup: shown as text); empty or the answer button → the default, none → nothing stored
					const value = env.cleanText(typed ?? '', limit) || env.cleanText(fallback ?? '', limit);
					// the question's text goes – unless a slice-trigger run took the box meanwhile (its message is shown)
					if (!deps.yieldToRuns || !vn.runHolds()) vn.hideMessage();
					if (!value) break;
					if (scope === 'reader') await env.setReaderName(value).catch((e) => console.warn('VN: the reader name was not stored', e));
					else if (variable) game.setVariable(variable, value);
					break;
				}

				case 'hideMessage':
					vn.hideMessage();
					break;

				case 'addChoice':
					vn.addChoice({ label: show(event.params.label, i - 1), onSelect: event.params.onSelect, from: label() });
					unshown++;
					break;

				case 'showChoices': {
					unshown = 0;
					// every choice hidden by its condition: nothing to ask – the run goes on (no question without a choice)
					if (!vn.showChoices()) {
						console.warn('VN: showChoices without a visible choice – going on');
						break;
					}
					jumpsWithoutWait = 0;
					await vn.whenChoicesClosed();
					if (vn.isNavigationTriggered()) return 'navigated';
					const to = vn.takePendingDivert();
					if (to) divertTo(to);
					break;
				}

				case 'joinScene':
					vn.joinScene(event.params.characters);
					break;

				case 'leaveScene':
					vn.leaveScene(event.params.characters);
					break;

				case 'showTextBox': {
					const { wait = false } = event.params;
					if (!vn.showTextBox(event.params, deps.scope ?? '')) break;
					if (wait) {
						jumpsWithoutWait = 0;
						await new Promise<void>((resolve) => vn.setTextBoxResolver(resolve));
					}
					break;
				}

				case 'fx': {
					// an impact – it ends by itself, so waiting for it is never a dead end
					const ms = vn.playFx?.(event.params, deps.scope ?? '') ?? 0;
					if (event.params.wait && ms > 0) {
						jumpsWithoutWait = 0;
						await new Promise((resolve) => setTimeout(resolve, ms));
						if (cancelled()) return 'cancelled';
					}
					break;
				}

				case 'jumpTo': {
					const { target, wait = false } = event.params;
					const url = buildJumpToUrl(target, game.state.toonId, label());
					const problem = url ? (deps.jumpProblem?.(target) ?? null) : 'not a jump target';
					if (!url || problem) {
						// a jump into nothing stays: the run goes on, the page ends with the follow-up bar (validation refuses it)
						console.warn(`VN: jumpTo stays – ${problem}`, target);
						break;
					}
					// one click per message: the click on the last message is the one that jumps (no extra click)
					if (wait) await vn.waitForContinue();
					if (cancelled()) return 'cancelled';
					deps.navigate(url);
					return 'navigated';
				}

				default:
					console.warn(`Unknown event type: ${(event as { type?: string }).type}`);
			}
		}
	} catch (error) {
		console.error(error);
	}
	if (unshown) {
		console.warn(`VN: ${unshown} choice(s) added but never shown – add a showChoices`);
		vn.dropUnshownChoices(unshown);
	}
	return cancelled() ? 'cancelled' : 'done';
}

/** what a trigger run needs of the box besides the runner's (box-state.ts) */
export interface TriggerVn extends RunnerVn {
	acquireRun(): Promise<number>;
	holds(id: number): boolean;
	interrupt(by: number): void;
	endRun(id: number): void;
	endOfEvents(): void;
}

/**
 * A slice trigger's / placed item's run (event-processor.ts wires it): it takes the box in turn (`acquireRun` – one run
 * at a time, not while a question / choices are open), parks the script's waiting message and ALWAYS gives it back
 * (`endRun` in `finally` – also after an error). One `divert` = the group's script from that point. Returns the URL to
 * navigate to, or null.
 */
export async function runTrigger(
	own: readonly VNEvent[],
	deps: Omit<CoreDeps, 'vn' | 'navigate' | 'yieldToRuns'> & { vn: TriggerVn; script: readonly VNEvent[]; label: string | null }
): Promise<string | null> {
	const { vn } = deps;
	const divert = own.length === 1 && own[0]?.type === 'divert' ? own[0].params?.to : null;
	const script = divert ? deps.script : own;
	const start = divert ? labelIndex(script, divert) : 0;
	if (divert && start < 0) {
		console.warn(`VN: no point "${divert}" in the group's script`);
		return null;
	}
	const id = await vn.acquireRun();
	let url: string | null = null;
	try {
		if (!vn.holds(id)) return null; // the page went away while it waited
		// who reads is known first: a group script that waits for the session shows its message before this run parks it
		await deps.env.readerKnown();
		vn.interrupt(id);
		const result = await runEvents(script, start, {
			...deps,
			vn,
			navigate: (to) => (url = to),
			currentLabel: () => deps.label,
			origin: divert ? 'script' : deps.origin
		});
		if (result === 'done') vn.endOfEvents();
	} finally {
		vn.endRun(id);
	}
	return url;
}
