// Ownership claims in the admin (agents/ITEMS.md "Ownership, claims, gifting"): status words and badge tones, the
// open count for the sidebar, and the item choice of a `422 choose_item` answer.
import type { ClaimStatus, OwnershipTransfer } from '@scutoons/api';
import { admin, ApiError } from '$lib/api.ts';
import { m } from '$lib/paraglide/messages.js';

export const CLAIM_STATUSES: ClaimStatus[] = ['open', 'transferred', 'rejected', 'withdrawn'];

const LABELS: Record<ClaimStatus, () => string> = {
	open: () => m['claims.status_open'](),
	transferred: () => m['claims.status_transferred'](),
	rejected: () => m['claims.status_rejected'](),
	withdrawn: () => m['claims.status_withdrawn']()
};

export const claimStatusLabel = (status: string) => LABELS[status as ClaimStatus]?.() ?? status;

/** Filled badge tones (DESIGN.md "Brutalist tools"): open = attention, transferred = done, rejected = error, withdrawn = grey */
export const claimTone = (status: string) =>
	status === 'open' ? 'warning' : status === 'transferred' ? 'published' : status === 'rejected' ? 'error' : 'draft';

const KINDS: Record<OwnershipTransfer['kind'], () => string> = {
	claim: () => m['claims.kind_claim'](),
	gift: () => m['claims.kind_gift'](),
	admin: () => m['claims.kind_admin']()
};

export const transferKindLabel = (kind: string) => KINDS[kind as OwnershipTransfer['kind']]?.() ?? kind;

/** One item (+ variant) the code can be claimed as, from a `422 choose_item` answer (`{ options: [{ item_id, variants }] }`) */
export interface ItemChoice {
	item_id: string;
	variant: string | null;
}

/** The choices of a `422 choose_item` error, else null */
export function itemChoices(error: unknown): ItemChoice[] | null {
	if (!(error instanceof ApiError) || error.code !== 'choose_item') return null;
	const options = (error.body?.options ?? []) as { item_id: string; variants: string[] | null }[];
	return options.flatMap((o): ItemChoice[] => (o.variants?.length ? o.variants.map((v) => ({ item_id: o.item_id, variant: v })) : [{ item_id: o.item_id, variant: null }]));
}

export const choiceLabel = (c: ItemChoice) => (c.variant ? `${c.item_id} · ${c.variant}` : c.item_id);

// ── the open count (sidebar badge, list header) ──
let openCount = $state<number | null>(null);

export const openClaims = {
	get count() {
		return openCount;
	},
	set(count: number) {
		openCount = count;
	},
	/** one tiny list call; a failure leaves the badge as it was (e.g. before the server's migration) */
	refresh() {
		return admin
			.listClaims({ status: 'open', page_size: 1 })
			.then((r) => (openCount = r.open))
			.catch(() => {});
	}
};
