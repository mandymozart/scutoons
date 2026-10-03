<script lang="ts">
	import { toonRelativePath } from '@scutoons/api';
	import { DEFAULT_SLICE, getAt, IMAGE_QUALITY_DENSITIES, MEDIA_QUALITIES, type Path, type QualityVariants } from '@scutoons/content';
	import { confirmDialog } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { chapterFolder, chapterSlug } from '$lib/editor/defaults.ts';
	import { toWebp, uploadOrReplace } from '$lib/editor/media-upload.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Import slices …" (owner 2026-10-03: "import slices like 26 to 108, upload them and place each on its own slice –
	 * to set up a chapter quickly"). The chosen images, sorted by the number in their name (`…-slice-26` → 26, which stays
	 * the slice number), go into the chapter's folder by the content rules: `<toon>/s1/c<n>/s1c<n>-slice-<N>.webp` (1x,
	 * 390 px wide) plus every density below the delivered one (owner 2026-10-03: "always generate the smaller ones"):
	 * a 3x file (`….3x.jpg`, `…@3x.png` or 1170 px wide) → `.webp`, `.2x.webp`, `.3x.webp`, all WebP, and the slice's
	 * `imageVariants` (low / medium / high) point at them – a missing level falls back to the next lower one in the reader.
	 * Several files for one slice (`slice-26.jpg` + `slice-26.3x.jpg`): each fills its own density, the rest is scaled
	 * from the biggest.
	 *
	 * New versions (owner 2026-10-03): a number the group already has (its image is `s1c<n>-slice-<N>…`) updates that
	 * slice – the files are replaced in place (the old bytes archived as an asset version, the CDN purged), the slice keeps
	 * its text boxes, items, events and audio. A new height is asked per slice (update anyway / skip). Other numbers are
	 * added at the end of the group, with their real height. One undo step for all.
	 */
	let {
		doc,
		groupPath,
		chapterNumber,
		season = 1,
		onselect
	}: { doc: ContentDoc; groupPath: Path; chapterNumber: number; season?: number; onselect: (index: number) => void } =
		$props();

	let input = $state<HTMLInputElement>();
	let progress = $state<{ done: number; total: number } | null>(null);
	let problems = $state<string[]>([]);
	let summary = $state('');

	/** "….3x.jpg", "…@2x.png", "… 3x.webp" → the density named in a file name; else null */
	const DENSITY_IN_NAME = /(?:^|[._@\- ])([123])x(?=[._\- ]|$)/i;
	const stem = (name: string) => name.replace(/\.(jpe?g|png|webp)$/i, '');
	const densityInName = (name: string) => {
		const found = stem(name).match(DENSITY_IN_NAME);
		return found ? Number(found[1]) : null;
	};

	/** the slice number in a file name (a density marker is not a number): "s1c1-slice-26.3x.jpg" → 26; else null */
	const numberOf = (name: string) => {
		const bare = stem(name).replace(DENSITY_IN_NAME, '');
		const found = bare.match(/slice[-_ ]?(\d+)/i) ?? bare.match(/(\d+)(?!.*\d)/);
		return found ? Number(found[1]) : null;
	};

	/** the density of a delivered image: from its name, else from its width (1170 px → 3x, 780 px → 2x) */
	const densityOf = (name: string, width: number) =>
		densityInName(name) ?? Math.max(1, Math.min(3, Math.floor((width + 1) / DEFAULT_SLICE.width)));

	/** the file path of a density: 1x = no suffix */
	const pathOf = (base: string, density: number) => `${base}${density > 1 ? `.${density}x` : ''}.webp`;

	/** the slice of the group showing `s1c<n>-slice-<N>` (also from an older folder: the same file name); -1 = none */
	const findSlice = (slices: any[], name: string) =>
		slices.findIndex((slice) => typeof slice?.image === 'string' && stem(slice.image.split('/').pop() ?? '').replace(DENSITY_IN_NAME, '') === name);

	type Imported = { image: string; width: number; height: number; imageVariants?: QualityVariants };

	async function importFiles(event: Event) {
		const files = [...((event.currentTarget as HTMLInputElement).files ?? [])];
		if (input) input.value = '';
		if (!files.length) return;
		// one entry per slice number (a slice may come as several density files)
		const bySlice = new Map<number, File[]>();
		files.forEach((file, i) => {
			const number = numberOf(file.name) ?? i + 1;
			bySlice.set(number, [...(bySlice.get(number) ?? []), file]);
		});
		const sorted = [...bySlice].sort(([a], [b]) => a - b);
		const folder = `${doc.toon}/${chapterFolder(chapterNumber, season)}`;
		const slug = chapterSlug(chapterNumber, season);
		const added: Imported[] = [];
		const updated: { name: string; slice: Imported }[] = [];
		let skipped = 0;
		problems = [];
		summary = '';
		progress = { done: 0, total: sorted.length };
		for (const [number, group] of sorted) {
			const bitmaps: { bitmap: ImageBitmap; density: number }[] = [];
			try {
				for (const file of group) {
					const bitmap = await createImageBitmap(file);
					bitmaps.push({ bitmap, density: densityOf(file.name, bitmap.width) });
				}
				bitmaps.sort((a, b) => b.density - a.density || b.bitmap.width - a.bitmap.width);
				const top = bitmaps[0].density;
				const name = `${slug}-slice-${number}`;
				const base = `${folder}/${name}`;
				// every density first: the 1x size decides whether an update changes the slice's height
				const encoded: { density: number; blob: Blob; width: number; height: number }[] = [];
				for (let density = 1; density <= top; density++) {
					// the file delivered for this density, else the smallest bigger one (the biggest is always >= density)
					const source = bitmaps.find((b) => b.density === density) ?? [...bitmaps].reverse().find((b) => b.density > density)!;
					encoded.push({ density, ...(await toWebp(source.bitmap, DEFAULT_SLICE.width * density)) });
				}
				const { width, height } = encoded[0];
				const existing: any[] = doc.get(doc.primary, [...groupPath, 'slices']) ?? [];
				const old = existing[findSlice(existing, name)];
				if (old && Number(old.height) && Math.abs(Number(old.height) - height) > 1) {
					const update = await confirmDialog({
						title: m['creator.import_slices_height_title']({ number }),
						message: m['creator.import_slices_height_message']({ old: Number(old.height), height }),
						confirmLabel: m['creator.import_slices_height_update'](),
						cancelLabel: m['creator.import_slices_height_skip']()
					});
					if (!update) {
						skipped++;
						continue;
					}
				}
				const variants: QualityVariants = {};
				for (const file of encoded) {
					await uploadOrReplace(file.blob, pathOf(base, file.density), 'slice import');
					const level = MEDIA_QUALITIES.find((q) => IMAGE_QUALITY_DENSITIES[q] === file.density);
					if (level) variants[level] = toonRelativePath(pathOf(base, file.density), doc.toon);
				}
				const slice: Imported = { image: toonRelativePath(pathOf(base, 1), doc.toon), width, height, ...(top > 1 ? { imageVariants: variants } : {}) };
				if (old) updated.push({ name, slice });
				else added.push(slice);
			} catch (e) {
				problems = [...problems, `${group.map((f) => f.name).join(', ')}: ${e instanceof Error ? e.message : String(e)}`];
			} finally {
				for (const { bitmap } of bitmaps) bitmap.close();
				progress = { done: (progress?.done ?? 0) + 1, total: sorted.length };
			}
		}
		if (added.length || updated.length) {
			let first = -1;
			doc.structural((file) => {
				const group = getAt(file, groupPath) ?? file;
				if (!Array.isArray(group.slices)) group.slices = [];
				for (const { name, slice } of updated) {
					const at = findSlice(group.slices, name);
					if (at < 0) continue;
					const target = group.slices[at];
					Object.assign(target, { image: slice.image, width: slice.width, height: slice.height });
					if (slice.imageVariants) target.imageVariants = { ...slice.imageVariants };
					else delete target.imageVariants;
					if (first < 0) first = at;
				}
				if (added.length) {
					first = group.slices.length;
					group.slices.push(...added.map((slice) => ({ ...slice, ...(slice.imageVariants ? { imageVariants: { ...slice.imageVariants } } : {}) })));
				}
			});
			if (first >= 0) onselect(first);
		}
		if (updated.length || skipped) summary = m['creator.import_slices_summary']({ added: added.length, updated: updated.length, skipped });
		progress = null;
	}
</script>

<div class="slice-import">
	<button type="button" class="secondary" disabled={!!progress} onclick={() => input?.click()} title={m['creator.import_slices_hint']()} data-testid="slice-import">
		{progress ? m['creator.import_slices_progress']({ done: progress.done, total: progress.total }) : m['creator.import_slices']()}
	</button>
	<input bind:this={input} type="file" accept="image/jpeg,image/png,image/webp" multiple hidden onchange={importFiles} />
	{#if summary}<p class="sc-caption" role="status">{summary}</p>{/if}
	{#each problems as problem}
		<p class="error" role="alert">{problem}</p>
	{/each}
</div>

<style>
	.slice-import {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-top: 0.5rem;
	}
</style>
