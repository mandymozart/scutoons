import type { GameContext } from '$lib/contexts/game.svelte';
import { interpolateText } from '@scutoons/content';

/**
 * Game variables in text: `{name}` → its value, `{time:clock}` → minutes as a clock (5:30); unknown ones stay as written
 * @example "Hello {playerName}!" → "Hello Alice!"
 */
export function interpolateVariables(text: string, game: GameContext): string {
	return interpolateText(text, (name) => game.getVariable(name));
}
