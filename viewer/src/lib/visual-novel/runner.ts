import { labelIndex, testCondition, type VNEvent } from '@scutoons/content';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { buildJumpToUrl } from './jump-to.ts';
import { interpolateVariables } from './utils.ts';
import { executeVariableAction } from './variable-actions.ts';

/**
 * The one VN script runner (CUTOVER 10.27, owner 2026-10-03: "two runners" were a half-finished move – logic stays out of
 * the views again): the group's script (`interactions/visual-novel.svelte`) and slice / placed-item triggers
 * (`event-processor.ts`) both run events here. Story flow: `label` = a named point, `divert` (event or a picked choice)
 * continues there – no page change; `jumpTo` is navigation only (`navigate`, then the run ends); `if` skips an event;
 * `setVariable` writes a variable. A run that keeps diverting without waiting for the reader stops (a loop guard).
 */
export interface RunnerDeps {
	vn: VisualNovelContext;
	game: GameContext;
	/** go to another page (group / chapter); the run ends */
	navigate: (url: string) => void;
	/** the page went away (the group component was destroyed) */
	cancelled?: () => boolean;
	/** the group the reader is on (a random jump never picks it) */
	currentLabel?: () => string | null;
}

export type RunResult = 'done' | 'navigated' | 'cancelled';

/** diverts in a row without anything that waits for the reader before the run is stopped as a loop */
const LOOP_LIMIT = 200;

export async function runScript(events: readonly VNEvent[], start: number, deps: RunnerDeps): Promise<RunResult> {
	const { vn, game } = deps;
	const cancelled = () => deps.cancelled?.() ?? false;
	const label = () => deps.currentLabel?.() ?? null;
	const get = (name: string) => game.getVariable(name);
	let i = Math.max(0, start);
	let jumpsWithoutWait = 0;

	/** continue at a named point (false: there is none – the run goes on after the divert) */
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

	try {
		while (i < events.length) {
			if (cancelled()) return 'cancelled';
			const event = events[i];
			i++;
			if (!event || !testCondition(event.if, get)) continue;

			switch (event.type) {
				case 'label':
					break;

				case 'divert':
					divertTo(event.params?.to);
					break;

				case 'setVariable': {
					const { variable, value, op } = event.params;
					if (op === 'add') executeVariableAction({ add: { variable, value: Number(value) || 0 } }, game);
					else executeVariableAction({ set: { variable, value: String(value) } }, game);
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
					vn.showMessage(speaker, interpolateVariables(text, game), wait);
					if (wait) {
						jumpsWithoutWait = 0;
						await new Promise<void>((resolve) => vn.setWaitResolver(resolve));
					}
					break;
				}

				case 'hideMessage':
					vn.hideMessage();
					break;

				case 'addChoice':
					vn.addChoice({ label: interpolateVariables(event.params.label, game), onSelect: event.params.onSelect });
					break;

				case 'showChoices': {
					vn.showChoices();
					jumpsWithoutWait = 0;
					await new Promise<void>((resolve) => {
						const check = () => (vn.state.pendingChoices.length === 0 ? resolve() : setTimeout(check, 50));
						check();
					});
					if (vn.isNavigationTriggered()) return 'navigated';
					const to = vn.takePendingDivert();
					if (to) divertTo(to);
					break;
				}

				case 'joinScene':
					// refs are normalized by the context (new and old form, old ids, view default avatar)
					vn.joinScene(event.params.characters);
					break;

				case 'leaveScene':
					vn.leaveScene(event.params.characters);
					break;

				case 'showOnomatopeia': {
					const { wait = false } = event.params;
					if (!vn.showOnomatopeia(event.params)) break;
					if (wait) {
						jumpsWithoutWait = 0;
						await new Promise<void>((resolve) => vn.setOnomatopoeiaResolver(resolve));
					}
					break;
				}

				case 'jumpTo': {
					const { target, wait = false } = event.params;
					const url = buildJumpToUrl(target, game.state.toonId, label());
					if (!url) {
						console.warn('Invalid jumpTo target', target);
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
	return cancelled() ? 'cancelled' : 'done';
}
