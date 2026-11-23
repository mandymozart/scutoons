// "Is the version I hold still the newest?" (server/docs/API.md "Is there a newer version", RULES H19): the creator's
// editors and the deck's show ask the same conditional GET – `?only=version` on the item's own route.
import { ApiError, type ApiClient } from './client.ts';

export type NewestAnswer = { state: 'same' } | { state: 'newer'; version: number };

/** the item route's path: `/entities/<key>`, `/scenes/<id>` or `/shows/<id>` (the route decides who may ask) */
export function itemPath(key: string): string {
	if (key.startsWith('show:')) return `/shows/${key.slice(5)}`;
	if (key.startsWith('scene:')) return `/scenes/${key.slice(6)}`;
	return `/entities/${key}`;
}

/**
 * The newest version of `key` against `held` (the version id the client loaded or last saved): 304 = `same`, else the
 * server's id. Errors (401, 403, 404, network) are thrown as ApiError – a caller that only shows a notice ignores them.
 */
export async function newestVersion(client: ApiClient, key: string, held: number, signal?: AbortSignal): Promise<NewestAnswer> {
	try {
		const response = await client.fetchRaw(`${itemPath(key)}?only=version`, { headers: { 'If-None-Match': `"${held}"` }, signal });
		const body = (await response.json()) as { version?: unknown };
		const version = Number(body?.version);
		if (!Number.isInteger(version)) throw new ApiError(response.status, 'invalid_answer', 'The answer names no version.', body);
		return version === held ? { state: 'same' } : { state: 'newer', version };
	} catch (error) {
		if (error instanceof ApiError && error.status === 304) return { state: 'same' };
		throw error;
	}
}
