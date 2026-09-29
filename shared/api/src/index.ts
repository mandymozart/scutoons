// @scutoons/api: typed, framework-agnostic client for the SCUTOONS API. The app injects base URL and token getter.
export { ApiError, createApiClient, type ApiClient, type ApiClientOptions, type Method, type RequestOptions } from './client.ts';
export { SESSION_STORAGE_KEY, DEVICE_STORAGE_KEY, getDeviceId, getSessionToken, setSessionToken } from './storage.ts';
export {
	createQrApi,
	type QrApi,
	type VerifyResult,
	type AccessResult,
	type InventoryResult,
	type ProgressResult
} from './qr.ts';
export {
	CONTENT_PATH,
	createContentApi,
	type ContentApi,
	type ContentApiOptions,
	type ContentOptions,
	type ContentVersion,
	type WrappedContent,
	type ContentIndexFile
} from './content.ts';
