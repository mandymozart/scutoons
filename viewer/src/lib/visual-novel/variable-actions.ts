import type { GameContext } from '$lib/contexts/game.svelte';
import type { VNVariableSetter, VNVariableAdder } from '$lib/models/visual-novel.ts';

type VariableAction = VNVariableSetter | VNVariableAdder;

export function executeVariableAction(action: VariableAction, gameContext: GameContext): void {
	if ('set' in action) {
		gameContext.setVariable(action.set.variable, action.set.value);
		console.log(`Set ${action.set.variable} = ${action.set.value}`);
	} else if ('add' in action) {
		gameContext.addVariable(action.add.variable, action.add.value);
		console.log(`Add ${action.add.variable} += ${action.add.value}`);
	}
}
