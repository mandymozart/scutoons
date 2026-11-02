// The module's document `module:<id>` in the creator (#281 / #282): the designer's ModuleStore on the creator's content path –
// `GET /entities/module:<id>?draft=1` and `PUT` (a new draft version, the history keeps the old ones; publish is the
// catalog documents' own). It holds the version the next save builds on (a conflict answers 409, not a silent overwrite).
import { failureOf, readModuleContent, type Failure, type ModuleStore } from '@scutoons/rig-designer';
import { ApiError, getDraft, putDraft } from '$lib/api.ts';

type Drafts = { getDraft: typeof getDraft; putDraft: typeof putDraft };

const fail = (e: unknown): Failure => {
	if (e instanceof ApiError) return failureOf(e.status, { ...(typeof e.body === 'object' && e.body ? e.body : {}), error: e.code, message: e.message });
	return failureOf(0, { error: 'network', message: e instanceof Error ? e.message : String(e) });
};

export const moduleKey = (id: string) => `module:${id}`;

/** the draft store for `module:<id>`; `drafts` is the creator's api, tests pass fakes */
export function createModuleStore(drafts: Drafts = { getDraft, putDraft }): ModuleStore {
	const base = new Map<string, number | null>();
	return {
		async load(id) {
			try {
				const wrapped = await drafts.getDraft(moduleKey(id));
				base.set(id, wrapped.version.id);
				return { ok: true, module: readModuleContent(wrapped.content, id) };
			} catch (e) {
				if (e instanceof ApiError && e.status === 404) {
					base.set(id, null);
					return { ok: true, module: null };
				}
				return fail(e);
			}
		},
		async save(id, content) {
			try {
				const r = await drafts.putDraft(moduleKey(id), content, base.get(id) ?? null);
				base.set(id, r.version.id);
				return { ok: true };
			} catch (e) {
				if (e instanceof ApiError && e.status === 422) {
					const problems = Array.isArray(e.body?.problems) ? e.body.problems : [];
					return failureOf(422, { error: e.code, message: e.message, problems });
				}
				return fail(e);
			}
		},
		compiled(id, version) {
			base.set(id, version);
		}
	};
}
