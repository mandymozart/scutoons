// @scutoons/media: the media tooling of the tools (creator, admin) – agents/COMPONENTS.md "Shared media components".
// The app calls setupMedia({ assets, cdn, locale }) once in its root layout and renders @scutoons/ui's <DialogHost />
// (the picker's "Replace the file?" is an in-page confirmDialog).
export { setupMedia, mediaAssets, mediaUrl, mediaText, type MediaSetup } from './setup.ts';
export { MEDIA_LABELS, mediaLabels, type MediaLabels } from './labels.ts';
export { default as AssetPickerSheet, bytes, type PickedAsset, type AssetPickerKind } from './asset-picker-sheet.svelte';
export { default as MediaSlot } from './media-slot.svelte';
export { default as MediaVariantsSlot } from './media-variants-slot.svelte';
export { default as FileSlot } from './file-slot.svelte';
export { toWebp, uploadOrReplace } from './media-upload.ts';
export {
	checkMedia,
	densityFromWidth,
	densityLevel,
	densityPath,
	densityPlan,
	IMAGE_MAX_BYTES,
	mediaKind,
	mediaType,
	splitMediaName,
	standardDensities,
	standardImage,
	standardName,
	toStandard,
	type Density,
	type MediaFacts,
	type MediaKind,
	type MediaVerdict,
	type StandardImage
} from './media-standard.ts';
export {
	cleanVariants,
	defaultLevel,
	mediaFromAsset,
	preferredVariant,
	withVariant,
	type VariantMedia
} from './media-variants.ts';
