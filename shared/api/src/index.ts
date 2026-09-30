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
export { createProductsApi, type ProductsApi } from './products.ts';
export {
	EVENT_NAMES,
	createAdminApi,
	toQuery,
	type AdminApi,
	type AdminCatalog,
	type AdminCodeDetail,
	type AdminCodeRow,
	type AdminContentFile,
	type AdminContentIndex,
	type AdminEventRow,
	type AdminKeys,
	type AdminSetting,
	type AdminSettings,
	type AdminUserDetail,
	type AdminCharacterUnlock,
	type AdminUserRow,
	type BatchManifest,
	type BatchManifestCode,
	type BatchRow,
	type BatchStep,
	type CodeEvent,
	type CodeSignature,
	type CodeSignatureResult,
	type CreateCodeInput,
	type CreatedCode,
	type Dashboard,
	type DashboardEdition,
	type EventCounts,
	type EventName,
	type FileType,
	type ListParams,
	type ListResult,
	type Pagination,
	type RoleInfo
} from './admin.ts';
