// The viewer's API wiring: @scutoons/api configured with this app's API base, static files and session token.
import {
	ApiError,
	createApiClient,
	createLibraryApi,
	createProductsApi,
	createQrApi,
	getDeviceId,
	getSessionToken
} from '@scutoons/api';
import { API_URL } from '$lib/config.ts';
import { draftPreview } from '$lib/draft-preview.svelte.ts';

export {
	ApiError,
	getDeviceId,
	type AccessResult,
	type InventoryResult,
	type ProgressResult,
	type VerifyResult
} from '@scutoons/api';

// credentials: the single sign-on cookie of every SCUTOONS app (@scutoons/auth); a token stored by older builds still
// goes as Bearer until the session has moved it into the cookie
export const api = createApiClient({ baseUrl: API_URL, getToken: getSessionToken, getDeviceId, credentials: 'include' });

/** the account calls of the shared session (@scutoons/auth `createAuthSession`, contexts/session.svelte.ts) */
export const authApi = createQrApi(api);

export const {
	checkAccess,
	updateMe,
	fetchCode,
	claimCode,
	craftRecipe,
	grantItem,
	importItems,
	putAvatar,
	putAvatarBody,
	useItem,
	endEffect,
	fetchProgress,
	putProgress,
	// ownership + gifting (agents/ITEMS.md "Ownership, claims, gifting")
	giftItem,
	markReceivedSeen,
	fileClaim,
	addClaimFile,
	myClaims,
	withdrawClaim
} = createQrApi(api);

/** Public data: a client without session / device id headers, so the browser sends a simple request (no CORS preflight). */
const publicApi = createApiClient({ baseUrl: API_URL });

/** Shop data (homepage slides): API /products/ and /products/featured/; the stores carrying them: /stockists/ ("Where to buy"). */
export const { fetchProducts, fetchFeaturedProducts, fetchStockists } = createProductsApi(publicApi);

/** Item catalog + editions (public; the inventory loads them on every page). */
export const { fetchCatalog, fetchEditions } = createQrApi(publicApi);

/**
 * The reader's content from the API (library endpoints, agents/DATA.md): the toon page in one request, one request per
 * chapter (scenes inlined), cast, credits, events. Public answers are kept 60 s, so the chapter page and the layout share
 * requests (agents/PERFORMANCE.md).
 */
export const library = createLibraryApi({ publicClient: publicApi, sessionClient: api, cacheMs: 60_000 });
export const { fetchCharacters, fetchRegistry, fetchCredits, fetchEvents, fetchMap } = library;

/**
 * Toons, a toon, a chapter – with a creator's draft preview on (lib/draft-preview.svelte.ts) as `?draft=1`. When the
 * session no longer has the role (401 unauthorized / 403 forbidden), the preview switches off and the public answer is used.
 */
async function preview<T>(load: (draft: boolean) => Promise<T>): Promise<T> {
	if (!draftPreview.on) return load(false);
	try {
		return await load(true);
	} catch (e) {
		if (e instanceof ApiError && (e.code === 'unauthorized' || e.code === 'forbidden')) {
			draftPreview.set(false);
			return load(false);
		}
		throw e;
	}
}
export const fetchToons = (locale?: string) => preview((draft) => library.fetchToons({ draft }, locale));
export const fetchToon = (toon: string, locale?: string) => preview((draft) => library.fetchToon(toon, locale, { draft }));
export const fetchChapter = (toon: string, number: number, locale?: string) =>
	preview((draft) => library.fetchChapter(toon, number, locale, { draft }));
