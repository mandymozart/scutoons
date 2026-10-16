import type { ParamMatcher } from '@sveltejs/kit';
import { isAccountGroup } from '$lib/account/groups.ts';

/** `/account/<group>`: only the account page's groups (lib/account/groups.ts) */
export const match: ParamMatcher = (param) => isAccountGroup(param);
