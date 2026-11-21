/**
 * Several documents go live together (POST /entities/publish, one transaction). Two answers of the server set single
 * documents aside without stopping the others: `review_required` names the keys that need a review (they are
 * submitted instead), `invalid_catalog` the one entry the catalog refuses (it is left out and named). The rest is
 * tried again, so what can go live still goes live together.
 */
export interface Version {
	key: string;
	version: number;
}

export interface Together {
	/** the keys that went live */
	done: string[];
	/** to submit for review */
	review: Version[];
	/** left out: the catalog refuses them, with the server's words */
	refused: { key: string; why: string }[];
}

type Refusal = { code?: unknown; message?: unknown; body?: { key?: unknown; keys?: unknown } | null };

export async function publishTogether(versions: Version[], publish: (versions: Version[]) => Promise<unknown>): Promise<Together> {
	const out: Together = { done: [], review: [], refused: [] };
	let live = versions;
	while (live.length) {
		try {
			await publish(live);
			out.done.push(...live.map((v) => v.key));
			break;
		} catch (e) {
			const error = (typeof e === 'object' && e !== null ? e : {}) as Refusal;
			const keys = error.code === 'review_required' && Array.isArray(error.body?.keys) ? error.body.keys : [];
			const refused = error.code === 'invalid_catalog' ? error.body?.key : undefined;
			const aside = live.filter((v) => keys.includes(v.key) || v.key === refused);
			// nothing of this batch is named: the error is not about single documents
			if (!aside.length) throw e;
			if (keys.length) out.review.push(...aside);
			else out.refused.push({ key: String(refused), why: String(error.message ?? '') });
			live = live.filter((v) => !aside.includes(v));
		}
	}
	return out;
}
