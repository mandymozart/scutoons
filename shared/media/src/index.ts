// @scutoons/media: the media tooling of the tools (creator, admin) – agents/COMPONENTS.md "Shared media components".
// The app calls setupMedia({ assets, cdn, locale }) once in its root layout and renders @scutoons/ui's <DialogHost />
// (an upload's "Replace the file?" is an in-page confirmDialog). A slot asks the app's `host` for a pick (slot-host.ts).
export { setupMedia, mediaAssets, mediaHost, mediaUrl, mediaText, type MediaSetup } from './setup.ts';
export { slotFit, type DropAnswer, type MediaHost, type PickedFile, type SlotApply, type SlotKind, type SlotPick } from './slot-host.ts';
export { MEDIA_LABELS, mediaLabels, type MediaLabels } from './labels.ts';
export { default as MediaSlot } from './media-slot.svelte';
export { default as MediaVariantsSlot } from './media-variants-slot.svelte';
export { default as FileSlot } from './file-slot.svelte';
export { default as ProtectedFileSlot } from './protected-file-slot.svelte';
export { default as ModelPreview } from './model-preview.svelte';
export { default as ModelThumb } from './model-thumb.svelte';
export { modelThumbnail, type ModelInfo } from './model-view.js';
export { toWebp, uploadOrReplace } from './media-upload.ts';
export { bestAudioSource, decodeAudio, encodeMp3, nearestKbps, planAudioUpload, type AudioUploadPlan } from './audio-levels.ts';
export {
	checkMedia,
	densityFromWidth,
	densityLevel,
	densityPath,
	densityPlan,
	densityWidth,
	mediaKind,
	mediaType,
	splitMediaName,
	standardDensities,
	standardImage,
	standardName,
	toStandard,
	type Density,
	type DensityWidths,
	type MediaFacts,
	type MediaKind,
	type MediaVerdict,
	type StandardImage
} from './media-standard.ts';
export { auditAsset, auditAssets, fixFindings, nextFreeName, type AuditAsset, type FixDeps, type FixReport, type FixRow, type StandardFinding } from './media-audit.ts';
export {
	cleanVariants,
	defaultLevel,
	mediaFromFiles,
	preferredVariant,
	withVariant,
	type VariantMedia
} from './media-variants.ts';
export {
	LIBRARY_GROUPINGS,
	LIBRARY_SCOPES,
	LIBRARY_SORTS,
	bytes,
	formatDuration,
	previewPicture,
	previewStill,
	type AssetFamily,
	type FamilyVersion,
	type LibraryCast,
	type LibraryCell,
	type LibraryContext,
	type LibraryEditor,
	type LibraryFitAnswer,
	type LibraryOpenKind,
	type LibraryOpening,
	type LibraryGroup,
	type LibraryGrouping,
	type LibraryKind,
	type LibraryOrigin,
	type LibraryPreview,
	type LibraryQuery,
	type LibraryScope,
	type LibrarySort,
	type LibraryUse,
	type LibraryView,
	type SlotFit
} from './library/types.ts';
