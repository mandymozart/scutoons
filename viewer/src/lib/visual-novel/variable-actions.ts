import type { GameContext } from '$lib/contexts/game.svelte';
import { getCurrentInventory } from '$lib/contexts/inventory.svelte.ts';
import { actionsOf, GIVE_ITEM_MAX, periodOf, repeatOf, type Repeat, type VNAction, type VNVariableSetter, type VNVariableAdder, type VNGiveItem, type VNJumpTo, type RecurrenceInterval } from '@scutoons/content';
import { pageView } from './page-view.ts';
import { buildJumpToUrl } from './jump-to.ts';

type VariableAction = VNVariableSetter | VNVariableAdder | VNGiveItem | VNJumpTo;

/**
 * The grant's `ref` = the place of the find (page without locale prefix + `?label=`): the server grants a ref once per
 * account, so each find counts once – replaying the same scene grants nothing, another find (another group) does.
 * `place` narrows it to one spot of the page (a placed item on a slice: its placement id).
 */
function storyRef(item: string, place?: string): string {
	const at = place ? `#${place}` : '';
	if (typeof location === 'undefined') return `vn:${item}${at}`;
	const path = location.pathname.replace(/^\/(de|en)(?=\/)/, '');
	const label = new URLSearchParams(location.search).get('label');
	return `vn:${item}:${path}${label ? `?label=${label}` : ''}${at}`;
}

/**
 * Give the reader a catalog item (`giveItem` / a choice's `give` – BACKLOG 10.38: an item is never a story variable):
 * source "story", once per place (the ref), the catalog's rules (grantable, max). null = no such item.
 */
export function giveStoryItem(
	item: string,
	qty = 1,
	place?: string,
	options?: { quietWhenNone?: boolean; respawn?: { respawn: RecurrenceInterval; visit?: string }; variant?: string | null }
): Promise<{ ok: boolean; error?: string; granted?: number }> | null {
	const inventory = getCurrentInventory();
	const n = Math.min(GIVE_ITEM_MAX, Math.max(1, Math.floor(Number(qty) || 1)));
	if (!inventory?.item(item)) return null;
	return inventory.grant(item, 'story', n, storyRef(item, place), options);
}

/** takes done on this page view (`view` repeat rules) */
const takenThisView = new Set<string>();

/**
 * A story takes an item (`takeItem` / a choice's `take` – owner 2026-10-07, BACKLOG 10.38). `repeat` = how often it
 * takes again (the trigger intervals; none = every time) – remembered in the save game (`took:<place>`, like a
 * pickup's `picked:`), the server takes once per ref (the period is part of it).
 * Returns `taken`, `done` (this period took already – the story goes on as if paid) or `refused` (too few, not
 * takeable: nothing taken – in a choice the actions after it don't run).
 */
export function takeStoryItem(take: { item: string; qty?: number; repeat?: Repeat }, game: GameContext): 'taken' | 'done' | 'refused' {
	const inventory = getCurrentInventory();
	if (!inventory) return 'refused';
	const qty = Math.min(GIVE_ITEM_MAX, Math.max(1, Math.floor(Number(take.qty) || 1)));
	const rule = repeatOf(take.repeat);
	const place = storyRef(take.item, `take${qty}`);
	const key = `took:${place}`;
	const period = rule.mode === 'view' ? periodOf('visit', Date.now(), pageView().id) : rule.mode === 'interval' && rule.every ? periodOf(rule.every, Date.now(), pageView().id) : null;
	if (rule.mode === 'once' && game.getVariable(key)) return 'done';
	if (rule.mode === 'view' && takenThisView.has(`${period}:${place}`)) return 'done';
	if (rule.mode === 'interval' && period && game.getVariable(key) === period) return 'done';
	const ref = rule.mode === 'every' ? undefined : period ? `${place}@${period}` : place;
	if (!inventory.take(take.item, qty, ref)) return 'refused';
	if (rule.mode === 'once') game.setVariable(key, 1);
	else if (rule.mode === 'view') takenThisView.add(`${period}:${place}`);
	else if (rule.mode === 'interval' && period) game.setVariable(key, period);
	return 'taken';
}

export function executeVariableAction(
	action: VariableAction,
	gameContext: GameContext,
	currentLabel: string | null = null,
	/** why a jump can't go there (vn context `jumpProblem`) – a jump into nothing is no jump: the script goes on */
	check?: (target: VNJumpTo['jumpTo']['target']) => string | null
): string | null {
	if ('set' in action) {
		gameContext.setVariable(action.set.variable, action.set.value);
		console.log(`Set ${action.set.variable} = ${action.set.value}`);
		return null;
	} else if ('add' in action) {
		gameContext.addVariable(action.add.variable, action.add.value);
		console.log(`Add ${action.add.variable} += ${action.add.value}`);
		return null;
	} else if ('give' in action) {
		giveStoryItem(action.give.item, action.give.qty, undefined, action.give.variant ? { variant: action.give.variant } : undefined);
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
		// a take that can't be paid stops the rest – a trade never gives for free (the story goes on)
		if ('take' in action) {
			if (takeStoryItem(action.take, gameContext) === 'refused') return { jumpTo: null, divert: null };
			continue;
		}
		const jumpTo = executeVariableAction(action, gameContext, currentLabel, check);
		if (jumpTo) return { jumpTo, divert: null };
	}
	return { jumpTo: null, divert };
}
