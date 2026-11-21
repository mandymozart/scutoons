// The History's state over the content store (#687): a document's saved versions as steps, what a step changed, a revert
// planned by the core and stored as a new draft. Every rule is deck-core's (`HistoryView`, through the wasm); what is
// here is fetching, caching and posting – shared by the creator's History panel, admin's and the MCP.
import type { HistoryBackTo, HistoryDescription, HistoryRevert, HistorySaved, HistoryStep, HistoryView } from '@scutoons/content';
import type { ContentApi, ContentVersion } from './content.ts';

/** a version as the core reads it */
export const savedOf = (versions: ContentVersion[]): HistorySaved[] =>
	versions.map((v) => ({ id: v.id, author: v.author_email ?? null, at_ms: Date.parse(v.created_at) || 0, message: v.message ?? null }));

/** the change `before` → `after` undone on version `base`, as the core plans it */
export interface RevertPlan<T = unknown> extends HistoryRevert<T> {
	base: number;
}

export interface KeyHistory {
	versions: ContentVersion[];
	steps: HistoryStep[];
}

export function createHistory(content: Pick<ContentApi, 'getVersion' | 'listVersions' | 'revertChange' | 'revertTo'>, core: () => Promise<HistoryView>) {
	/** a version's document (versions never change: kept once fetched) */
	const contents = new Map<string, Promise<unknown>>();
	const contentOf = (key: string, id: number): Promise<unknown> => {
		const at = `${key}@${id}`;
		let c = contents.get(at);
		if (!c) {
			c = content.getVersion(key, id).then((v) => v.content);
			contents.set(at, c);
			c.catch(() => contents.delete(at));
		}
		return c;
	};

	return {
		contentOf,
		core,

		async load(key: string): Promise<KeyHistory> {
			const [{ versions }, h] = await Promise.all([content.listVersions(key), core()]);
			return { versions, steps: h.steps(savedOf(versions)) };
		},

		/** what a step changed, in words (null: nothing before it to compare with) */
		async describe(key: string, step: HistoryStep): Promise<HistoryDescription | null> {
			if (step.before === null) return null;
			const [h, before, after] = await Promise.all([core(), contentOf(key, step.before), contentOf(key, step.last)]);
			return h.describe(before, after);
		},

		/** the step `before` → `after` undone on version `base` */
		async plan(key: string, before: number, after: number, base: number): Promise<RevertPlan> {
			const [h, a, b, now] = await Promise.all([core(), contentOf(key, before), contentOf(key, after), contentOf(key, base)]);
			return { ...h.revert(a, b, now), base };
		},

		/** stores a planned revert of version `version`'s step as a new draft; the new version's id (ApiError 409: saved since `base`) */
		async apply(key: string, version: number, plan: RevertPlan): Promise<number> {
			return (await content.revertChange(key, version, plan.doc, plan.base)).version.id;
		},

		/** back to version `version`: its document as a new draft */
		async backTo(key: string, version: number, base: number): Promise<number> {
			return (await content.revertTo(key, version, base)).version.id;
		},

		/** what going back to a point undoes: the later saves, the other people among them */
		async backToWords(versions: ContentVersion[], point: number, me: string): Promise<HistoryBackTo> {
			return (await core()).backTo(savedOf(versions), point, me);
		}
	};
}

export type History = ReturnType<typeof createHistory>;
