import type { GameContext } from '$lib/contexts/game.svelte';
import { getCurrentInventory } from '$lib/contexts/inventory.svelte.ts';
import type { VNVariableSetter, VNVariableAdder, VNJumpTo } from '@scutoons/content';
import { buildJumpToUrl } from './jump-to.ts';

type VariableAction = VNVariableSetter | VNVariableAdder | VNJumpTo;

/**
 * The grant's `ref` = the place of the find (page without locale prefix + `?label=`): the server grants a ref once per
 * account, so each find counts once – replaying the same scene grants nothing, another find (another group) does.
 */
function storyRef(variable: string): string {
	if (typeof location === 'undefined') return `vn:${variable}`;
	const path = location.pathname.replace(/^\/(de|en)(?=\/)/, '');
	const label = new URLSearchParams(location.search).get('label');
	return `vn:${variable}:${path}${label ? `?label=${label}` : ''}`;
}

/**
 * A VN variable whose name is a catalog item id (e.g. `cucumber`) also grants that item (source "story").
 * The game variable itself is kept for text interpolation.
 */
function grantStoryItem(variable: string, qty: number) {
	const inventory = getCurrentInventory();
	if (!inventory?.item(variable) || qty <= 0) return;
	inventory.grant(variable, 'story', qty, storyRef(variable));
}

export function executeVariableAction(action: VariableAction, gameContext: GameContext): string | null {
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
		const destination = buildJumpToUrl(action.jumpTo.target, gameContext.state.toonId);
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
