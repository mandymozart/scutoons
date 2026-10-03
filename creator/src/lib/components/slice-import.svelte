<script lang="ts">
	import { ApiError, toonRelativePath } from '@scutoons/api';
	import { DEFAULT_SLICE, getAt, type Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { chapterFolder, chapterSlug } from '$lib/editor/defaults.ts';
	import { assetsApi } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Import slices …" (owner 2026-10-03: "import slices like 26 to 108, upload them and place each on its own slice –
	 * to set up a chapter quickly"). The chosen images, sorted by the number in their name (`…-slice-26` → 26, which stays
	 * the slice number), go into the chapter's folder by the content rules: the browser scales each to the reader's 1x
	 * (390 px wide, WebP) → `<toon>/s1/c<n>/s1c<n>-slice-<N>.webp`; a 3x export (1170 px) also goes up as its `.3x`
	 * variant. Each becomes its own slice at the end of the group, with its real height – one undo step for all.
	 * Existing files are kept (409 exists): the slice points at them.
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

	const WEBP_QUALITY = 0.88;

	/** the slice number in a file name: "s1c1-slice-26.3x.jpg" → 26; else null */
	const numberOf = (name: string) => {
		const found = name.match(/slice[-_ ]?(\d+)/i) ?? name.match(/(\d+)(?!.*\d)/);
		return found ? Number(found[1]) : null;
	};

	async function toWebp(bitmap: ImageBitmap): Promise<{ blob: Blob; width: number; height: number }> {
		const width = Math.min(DEFAULT_SLICE.width, bitmap.width);
		const height = Math.round((bitmap.height * width) / bitmap.width);
		const canvas = document.createElement('canvas');
		canvas.width = width;
		canvas.height = height;
		const ctx = canvas.getContext('2d');
		if (!ctx) throw new Error('no canvas');
		ctx.imageSmoothingEnabled = true;
		ctx.imageSmoothingQuality = 'high';
		ctx.drawImage(bitmap, 0, 0, width, height);
		const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/webp', WEBP_QUALITY));
		if (!blob) throw new Error('WebP encoding failed');
		return { blob, width, height };
	}

	/** upload, keeping an existing file (true = there now) */
	async function put(blob: Blob, path: string): Promise<boolean> {
		try {
			await assetsApi.upload(blob, path);
			return true;
		} catch (e) {
			if (e instanceof ApiError && e.code === 'exists') return true;
			throw e;
		}
	}

	async function importFiles(event: Event) {
		const files = [...((event.currentTarget as HTMLInputElement).files ?? [])];
		if (input) input.value = '';
		if (!files.length) return;
		const sorted = files
			.map((file, i) => ({ file, number: numberOf(file.name) ?? i + 1 }))
			.sort((a, b) => a.number - b.number || a.file.name.localeCompare(b.file.name));
		const folder = `${doc.toon}/${chapterFolder(chapterNumber, season)}`;
		const slug = chapterSlug(chapterNumber, season);
		const slices: { image: string; width: number; height: number }[] = [];
		problems = [];
		progress = { done: 0, total: sorted.length };
		for (const { file, number } of sorted) {
			try {
				const bitmap = await createImageBitmap(file);
				const base = `${folder}/${slug}-slice-${number}`;
				const { blob, width, height } = await toWebp(bitmap);
				await put(blob, `${base}.webp`);
				// a 3x export also as its density variant (the reader serves 1x today; ASSETS.md "Folder convention")
				if (bitmap.width >= DEFAULT_SLICE.width * 3) {
					const ext = (file.name.match(/\.(jpe?g|png|webp)$/i)?.[1] ?? 'jpg').toLowerCase().replace('jpeg', 'jpg');
					await put(file, `${base}.3x.${ext}`);
				}
				bitmap.close();
				slices.push({ image: toonRelativePath(`${base}.webp`, doc.toon), width, height });
			} catch (e) {
				problems = [...problems, `${file.name}: ${e instanceof Error ? e.message : String(e)}`];
			}
			progress = { done: progress.done + 1, total: sorted.length };
		}
		if (slices.length) {
			let first = 0;
			doc.structural((file) => {
				const group = getAt(file, groupPath) ?? file;
				if (!Array.isArray(group.slices)) group.slices = [];
				first = group.slices.length;
				group.slices.push(...slices);
			});
			onselect(first);
		}
		progress = null;
	}
</script>

<div class="slice-import">
	<button type="button" class="secondary" disabled={!!progress} onclick={() => input?.click()} title={m['creator.import_slices_hint']()} data-testid="slice-import">
		{progress ? m['creator.import_slices_progress']({ done: progress.done, total: progress.total }) : m['creator.import_slices']()}
	</button>
	<input bind:this={input} type="file" accept="image/jpeg,image/png,image/webp" multiple hidden onchange={importFiles} />
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
