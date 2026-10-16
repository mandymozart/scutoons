import { m } from '$lib/paraglide/messages.js';
import type { AccountGroup } from '$lib/account/groups.ts';

/** a group's title, e.g. "Stammdaten" / "Account details" */
export const groupTitle = (group: AccountGroup): string => m[`account.group_${group}`]();
