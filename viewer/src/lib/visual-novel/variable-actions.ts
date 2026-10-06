import type { GameContext } from '$lib/contexts/game.svelte';
import { getCurrentInventory } from '$lib/contexts/inventory.svelte.ts';
import { actionsOf, type VNAction, type VNVariableSetter, type VNVariableAdder, type VNJumpTo, type RecurrenceInterval } from '@scutoons/content';
import { buildJumpToUrl } from './jump-to.ts';

type VariableAction = VNVariableSetter | VNVariableAdder | VNJumpTo;

/**
 * The grant's `ref` = the place of the find (page without locale prefix + `?label=`): the server grants a ref once per
 * account, so each find counts once – replaying the same scene grants nothing, another find (another group) does.
 * `place` narrows it to one spot of the page (a placed item on a slice: `<slice image>#<placement id>`).
 */
function storyRef(variable: string, place?: string): string {
	const at = place ? `#${place}` : '';
	if (typeof location === 'undefined') return `vn:${variable}${at}`;
	const path = location.pathname.replace(/^\/(de|en)(?=\/)/, '');
	const label = new URLSearchParams(location.search).get('label');
	return `vn:${variable}:${path}${label ? `?label=${label}` : ''}${at}`;
}

/**
 * A VN variable whose (plain) name is a catalog item id (e.g. `cucumber`) also grants that item (source "story").
 * The game variable itself is kept for text interpolation. Resolves to the inventory's answer (null = no such item).
 */
export function grantStoryItem(
	variable: string,
	qty: number,
	place?: string,
	options?: { quietWhenNone?: boolean; respawn?: { respawn: RecurrenceInterval; visit?: string } }
): Promise<{ ok: boolean; error?: string; granted?: number }> | null {
	const inventory = getCurrentInventory();
	// the plain name decides, whatever its domain (`cucumber`, `global.cucumber`, `toptoy.cucumber` – variables.ts)
	const item = variable.slice(variable.indexOf('.') + 1);
	if (!inventory?.item(item) || qty <= 0) return null;
	return inventory.grant(item, 'story', qty, storyRef(item, place), options);
}

export function executeVariableAction(
	action: VariableAction,
	gameContext: GameContext,
	currentLabel: string | null = null,
	/** why a jump can't go there (vn context `jumpProblem`) – a jump into nothing is no jump: the script goes on */
	check?: (target: VNJumpTo['jumpTo']['target']) => string | null
): string | null {
	if ('set' in action) {
		const previous = Number(gameContext.getVariable(action.set.variable)) || 0;
		gameContext.setVariable(action.set.variable, action.set.value);
		console.log(`Set ${action.set.variable} = ${action.set.value}`);
		const next = Number(action.set.value);
		// numeric set grants the difference, any other truthy value one piece
		grantStoryItem(
			action.set.variable,
			Number.isFinite(next) ? next - previous : action.set.value && action.set.value !== 'false' ? 1 : 0
		);
		return null;
	} else if ('add' in action) {
		gameContext.addVariable(action.add.variable, action.add.value);
		console.log(`Add ${action.add.variable} += ${action.add.value}`);
		grantStoryItem(action.add.variable, action.add.value);
		return null;
	} else if ('jumpTo' in action) {
		const problem = check?.(action.jumpTo.target);
		if (problem) {
			console.warn(`Choice: the jump stays – ${problem}`, action.jumpTo.target);
			return null;
		}
		const destination = buildJumpToUrl(action.jumpTo.target, gameContext.state.toonId, currentLabel);
		if (destination) {
			console.log(`Choice selected, will jump to: ${destination}`);
			return destination;
		} else {
			console.warn('Invalid jumpTo target', action.jumpTo.target);
			return null;
		}
	}
	return null;
}

/**
 * A picked choice's actions in order (CUTOVER 10.27: one action or a list, e.g. add 15 minutes, then jump): variables are
 * written, the first jump wins (the page changes – later actions don't run), a divert is handed back to the script.
 */
export function runChoiceActions(
	onSelect: VNAction | VNAction[] | undefined,
	gameContext: GameContext,
	currentLabel: string | null,
	check?: (target: VNJumpTo['jumpTo']['target']) => string | null
): { jumpTo: string | null; divert: string | null } {
	let divert: string | null = null;
	for (const action of actionsOf(onSelect)) {
		if ('divert' in action) {
			divert = action.divert.to;
			continue;
		}
		const jumpTo = executeVariableAction(action, gameContext, currentLabel, check);
		if (jumpTo) return { jumpTo, divert: null };
	}
	return { jumpTo: null, divert };
}
