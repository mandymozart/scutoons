import type { GameContext } from '$lib/contexts/game.svelte';
import type { VNVariableSetter, VNVariableAdder, VNJumpTo } from '$lib/models/visual-novel.ts';
import { buildJumpToUrl } from './jump-to.ts';

type VariableAction = VNVariableSetter | VNVariableAdder | VNJumpTo;

export function executeVariableAction(action: VariableAction, gameContext: GameContext): string | null {
	if ('set' in action) {
		gameContext.setVariable(action.set.variable, action.set.value);
		console.log(`Set ${action.set.variable} = ${action.set.value}`);
		return null;
	} else if ('add' in action) {
		gameContext.addVariable(action.add.variable, action.add.value);
		console.log(`Add ${action.add.variable} += ${action.add.value}`);
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
