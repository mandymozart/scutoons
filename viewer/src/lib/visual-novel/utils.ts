import type { GameContext } from '$lib/contexts/game.svelte';

/**
 * Interpolates game variables in text
 * @example "Hello {playerName}!" → "Hello Alice!"
 */
export function interpolateVariables(text: string, game: GameContext): string {
	return text.replace(/{(\w+)}/g, (match, varName) => {
		const value = game.getVariable(varName);
		if (value === undefined) {
			console.warn(`Variable "${varName}" not found in game state`);
			return match;
		}
		return String(value);
	});
}
