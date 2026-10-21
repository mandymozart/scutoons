<script lang="ts">
	import { ApiError, rootPath, toonRelativePath } from '@scutoons/api';
	import { DEFAULT_SLICE, getAt, type Path, type QualityVariants } from '@scutoons/content';
	import { confirmDialog, Icon, Sheet } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { chapterFolder, chapterSlug } from '$lib/editor/defaults.ts';
	import { densityLevel, densityPath, mediaAssets, splitMediaName, standardDensities, uploadOrReplace, type Density } from '@scutoons/media';
	import { m } from '$lib/paraglide/messages.js';
	import { replacedImages } from '$lib/editor/replaced-images.svelte.ts';

	/**
	 * "Import slices …" (owner 2026-10-03: "import slices like 26 to 108, upload them and place each on its own slice";
	 * 2026-10-06: a check list before the upload – "more deterministic, less guessing with file names"). The chosen
	 * images are listed first, one row per image: each row says which slice it replaces or that it becomes a new slice,
	 * and every row can be changed. Suggested: the slice whose image has the file's name (`s1c1-slice-26`, or just
	 * `slice-26` / `26` in this chapter), else a new slice. New slices go after / before the selected slice or at the end,
	 * in the list's order. Files with no number in the name never replace anything unless chosen.
	 *
	 * Files: WebP in every density up to the delivered one (owner 2026-10-03: "always generate the smaller ones"): a 3x
	 * file (`….3x.jpg`, `…@3x.png` or 1170 px wide) → `.webp`, `.2x.webp`, `.3x.webp`, and the slice's `imageVariants`
	 * (low / medium / high) point at them. Several files for one slice (`slice-26.jpg` + `slice-26.3x.jpg`) fill their
	 * own densities. A replaced slice keeps its file name, texts, items, events and audio (the old bytes are archived as
	 * an asset version, the CDN purged); a new height is asked per slice. A new slice gets a file name nothing uses yet
	 * (`s1c<n>-slice-<N>`, the file's number if free, else the next free one) – a new slice never overwrites a file.
	 * One undo step for all. Dropped from the computer onto the slice list (owner 2026-10-06): onto a slice = `replace`
	 * its image, between slices / before the first / after the last = `insert` new slices there.
	 */
	let {
		doc,
		groupPath,
		chapterNumber,
		season = 1,
		selected = -1,
		onselect
	}: {
		doc: ContentDoc;
		groupPath: Path;
		chapterNumber: number;
		season?: number;
		selected?: number;
		onselect: (index: number) => void;
	} = $props();

	type Target = 'new' | number;
	type Row = { key: string; label: string; files: File[]; preview: string; target: Target };
	type Placement = 'end' | `after:${number}` | `before:${number}`;
	type Imported = { image: string; width: number; height: number; imageVariants?: QualityVariants };

	let input = $state<HTMLInputElement>();
	let open = $state(false);
	let rows = $state<Row[]>([]);
	let placement = $state<Placement>('end');
	let progress = $state<{ done: number; total: number } | null>(null);
	let problems = $state<string[]>([]);
	let summary = $state('');

	const slices = $derived<any[]>(doc.get(doc.primary, [...groupPath, 'slices']) ?? []);
	const slug = $derived(chapterSlug(chapterNumber, season));
	const folder = $derived(`${doc.toon}/${chapterFolder(chapterNumber, season)}`);

	/** the name without extension and density marker: "s1c1-slice-26.3x.jpg" → "s1c1-slice-26" (the media standard's names) */
	const bareName = (name: string) => splitMediaName(name).stem;
	/** the slice number in a file name: "s1c1-slice-26.3x.jpg" → 26, "26.png" → 26; else null */
	const numberOf = (name: string) => {
		const bare = bareName(name);
		const found = bare.match(/slice[-_ ]?(\d+)/i) ?? bare.match(/(\d+)(?!.*\d)/);
		return found ? Number(found[1]) : null;
	};
	/** the file path of a density: 1x = no suffix */
	const pathOf = (base: string, density: Density) => densityPath(base, density);
	/** the CDN-root base path (no extension, no density) of a slice's image; null = none */
	const baseOf = (image: unknown) => (typeof image === 'string' && image ? rootPath(image.replace(/\.(jpe?g|png|webp)$/i, '').replace(/\.[123]x$/i, ''), doc.toon) : null);
	/** the number N of this chapter's `<slug>-slice-<N>` image; else null */
	const ownNumber = (image: unknown) => {
		if (typeof image !== 'string') return null;
		const found = bareName(image).match(new RegExp(`^${slug}-slice-(\\d+)$`));
		return found ? Number(found[1]) : null;
	};

	/** the suggested target of an image: the slice whose image has its name (exact, or this chapter's slice number) */
	function suggest(name: string, number: number | null): Target {
		const bare = bareName(name);
		const exact = slices.findIndex((s) => typeof s?.image === 'string' && bareName(s.image) === bare);
		if (exact >= 0) return exact;
		if (number === null) return 'new';
		const own = slices.findIndex((s) => ownNumber(s?.image) === number);
		return own >= 0 ? own : 'new';
	}

	/** files as rows in name order: one per slice number (several density files of one image), else one per name */
	function rowsOf(files: File[], preview: boolean): Row[] {
		const images = files.filter((f) => /^image\/(jpeg|png|webp)$/.test(f.type) || /\.(jpe?g|png|webp)$/i.test(f.name));
		const groups = new Map<string, File[]>();
		for (const file of images) {
			const number = numberOf(file.name);
			const key = number === null ? `name:${bareName(file.name).toLowerCase()}` : `n:${number}`;
			groups.set(key, [...(groups.get(key) ?? []), file]);
		}
		return [...groups]
			.map(([key, group]) => {
				const biggest = [...group].sort((a, b) => b.size - a.size)[0];
				const target = suggest(biggest.name, numberOf(biggest.name));
				return { key, label: group.map((f) => f.name).join(' + '), files: group, preview: preview ? URL.createObjectURL(biggest) : '', target };
			})
			.sort((a, b) => a.label.localeCompare(b.label, undefined, { numeric: true }));
	}

	/** the check list for the chosen files */
	export function choose(files: File[], where?: Placement) {
		const next = rowsOf(files, true);
		if (!next.length) return;
		closeRows();
		rows = next;
		placement = where ?? (selected >= 0 && selected < slices.length ? `after:${selected}` : 'end');
		problems = [];
		summary = '';
		open = true;
	}

	/** images dropped onto a slice: one replaces that slice's image; several open the check list (new ones after it) */
	export function replace(index: number, files: File[]) {
		const dropped = rowsOf(files, false);
		if (dropped.length === 1) void run([{ ...dropped[0], target: index }], 'end');
		else if (dropped.length) choose(files, `after:${index}`);
	}

	/** images dropped between slices (or before the first / after the last): new slices there, in name order */
	export function insert(at: number, files: File[]) {
		const dropped = rowsOf(files, false).map((row) => ({ ...row, target: 'new' as const }));
		if (dropped.length) void run(dropped, `before:${at}`);
	}

	function closeRows() {
		for (const row of rows) if (row.preview) URL.revokeObjectURL(row.preview);
		rows = [];
	}

	const replacing = $derived(rows.filter((r) => r.target !== 'new').length);
	const adding = $derived(rows.length - replacing);
	/** a slice chosen by two rows */
	const twice = $derived.by(() => {
		const seen = new Set<number>();
		for (const row of rows) {
			if (row.target === 'new') continue;
			if (seen.has(row.target)) return row.target;
			seen.add(row.target);
		}
		return null;
	});
	const sliceName = (index: number) => String(slices[index]?.image ?? '').split('/').pop() || '–';

	async function start() {
		if (twice !== null) return;
		const chosen = rows.map((r) => ({ ...r, files: [...r.files] }));
		const where = placement;
		open = false;
		await run(chosen, where);
		closeRows();
	}

	/**
	 * the files of one image in the media standard, every density up to the delivered one (1x first): a file's density
	 * from its name, else from its width (1170 px → 3x, 780 px → 2x)
	 */
	const encode = (files: File[]) => standardDensities(files, DEFAULT_SLICE.width);

	async function run(chosen: Row[], where: Placement) {
		const current = slices.map((s) => s?.image);
		// the numbers this chapter's slice files already use (a new slice takes a free one)
		const used = new Set<number>(current.map(ownNumber).filter((n): n is number => n !== null));
		const nextFree = (wanted: number | null) => {
			let n = wanted !== null && wanted > 0 && !used.has(wanted) ? wanted : Math.max(0, ...used) + 1;
			while (used.has(n)) n++;
			return n;
		};
		const added: Imported[] = [];
		const updated: { index: number; image: unknown; slice: Imported }[] = [];
		let skipped = 0;
		problems = [];
		summary = '';
		progress = { done: 0, total: chosen.length };
		for (const row of chosen) {
			try {
				const encoded = await encode(row.files);
				const { width, height } = encoded[0];
				const top = encoded[encoded.length - 1].density;
				const variants: QualityVariants = {};
				const record = (path: string, density: Density) => (variants[densityLevel(density)] = toonRelativePath(path, doc.toon));
				const old = row.target === 'new' ? null : slices[row.target];
				let base = old ? baseOf(old.image) : null;
				if (old && Number(old.height) && Math.abs(Number(old.height) - height) > 1) {
					const update = await confirmDialog({
						title: m['creator.import_slices_height_title']({ number: (row.target as number) + 1 }),
						message: m['creator.import_slices_height_message']({ old: Number(old.height), height }),
						confirmLabel: m['creator.import_slices_height_update'](),
						cancelLabel: m['creator.import_slices_height_skip']()
					});
					if (!update) {
						skipped++;
						continue;
					}
				}
				let rest = encoded;
				if (!base) {
					// a new file name: the 1x upload claims it (an existing file is never overwritten – take the next number)
					let number = nextFree(numberOf(row.files[0].name));
					for (;;) {
						const candidate = `${folder}/${slug}-slice-${number}`;
						try {
							await mediaAssets().upload(encoded[0].blob, pathOf(candidate, 1));
							base = candidate;
							break;
						} catch (e) {
							if (!(e instanceof ApiError && e.code === 'exists')) throw e;
							used.add(number);
							number = nextFree(null);
						}
					}
					used.add(number);
					record(pathOf(base, 1), 1);
					rest = encoded.slice(1);
				}
				for (const file of rest) {
					if ((await uploadOrReplace(file.blob, pathOf(base, file.density), 'slice import')) === 'replaced') replacedImages.mark(pathOf(base, file.density));
					record(pathOf(base, file.density), file.density);
				}
				const slice: Imported = { image: toonRelativePath(pathOf(base, 1), doc.toon), width, height, ...(top > 1 ? { imageVariants: variants } : {}) };
				if (old) updated.push({ index: row.target as number, image: current[row.target as number], slice });
				else added.push(slice);
			} catch (e) {
				problems = [...problems, `${row.label}: ${e instanceof Error ? e.message : String(e)}`];
			} finally {
				progress = { done: (progress?.done ?? 0) + 1, total: chosen.length };
			}
		}
		if (added.length || updated.length) {
			let first = -1;
			doc.edit((file) => {
				const group = getAt(file, groupPath) ?? file;
				if (!Array.isArray(group.slices)) group.slices = [];
				const list: any[] = group.slices;
				for (const { index, image, slice } of updated) {
					// the slice by its position, else (moved meanwhile) by its image
					const at = list[index]?.image === image ? index : list.findIndex((s) => s?.image === image);
					if (at < 0) continue;
					const target = list[at];
					Object.assign(target, { image: slice.image, width: slice.width, height: slice.height });
					if (slice.imageVariants) target.imageVariants = { ...slice.imageVariants };
					else delete target.imageVariants;
					if (first < 0) first = at;
				}
				if (added.length) {
					const [mode, ref] = where.split(':');
					const at = mode === 'end' ? list.length : Math.min(list.length, Math.max(0, Number(ref) + (mode === 'after' ? 1 : 0)));
					list.splice(at, 0, ...added.map((slice) => ({ ...slice, ...(slice.imageVariants ? { imageVariants: { ...slice.imageVariants } } : {}) })));
					first = at;
				}
			});
			if (first >= 0) onselect(first);
		}
		summary = m['creator.import_slices_summary']({ added: added.length, updated: updated.length, skipped });
		progress = null;
	}
</script>

<div class="slice-import">
	<button type="button" class="secondary" disabled={!!progress} onclick={() => input?.click()} title={m['creator.import_slices_hint']()} data-testid="slice-import">
		<Icon name="upload" />
		{progress ? m['creator.import_slices_progress']({ done: progress.done, total: progress.total }) : m['creator.import_slices']()}
	</button>
	<input
		bind:this={input}
		type="file"
		accept="image/jpeg,image/png,image/webp"
		multiple
		hidden
		onchange={(e) => {
			const files = [...(e.currentTarget.files ?? [])];
			e.currentTarget.value = '';
			choose(files);
		}}
	/>
	{#if summary}<p class="sc-caption" role="status">{summary}</p>{/if}
	{#each problems as problem}
		<p class="error" role="alert">{problem}</p>
	{/each}
</div>

<Sheet bind:open title={m['creator.import_slices_title']({ count: rows.length })} closeLabel={m['creator.cancel']()} onclose={() => { if (!progress) closeRows(); }}>
	<ol class="rows" data-testid="slice-import-rows">
		{#each rows as row, i (row.key)}
			<li>
				<img src={row.preview} alt="" />
				<span class="name" title={row.label}>{row.label}</span>
				<select bind:value={rows[i].target} aria-label={m['creator.import_slices_target']({ name: row.label })} data-testid="slice-import-target-{i}">
					<option value="new">{m['creator.import_slices_new']()}</option>
					{#each slices as _, index}
						<option value={index}>{m['creator.import_slices_replace']({ number: index + 1, name: sliceName(index) })}</option>
					{/each}
				</select>
			</li>
		{/each}
	</ol>
	{#if adding}
		<label class="where">
			{m['creator.import_slices_where']()}
			<select bind:value={placement} data-testid="slice-import-where">
				{#if selected >= 0 && selected < slices.length}
					<option value="after:{selected}">{m['creator.import_slices_after']({ number: selected + 1 })}</option>
					<option value="before:{selected}">{m['creator.import_slices_before']({ number: selected + 1 })}</option>
				{/if}
				{#if placement !== 'end' && placement !== `after:${selected}` && placement !== `before:${selected}`}
					<!-- a drop of several images: after the slice they were dropped on -->
					<option value={placement}>{m['creator.import_slices_after']({ number: Number(placement.split(':')[1]) + 1 })}</option>
				{/if}
				<option value="end">{m['creator.import_slices_end']()}</option>
			</select>
		</label>
	{/if}
	{#if twice !== null}<p class="error" role="alert">{m['creator.import_slices_twice']({ number: twice + 1 })}</p>{/if}
	{#snippet footer()}
		<button type="button" class="ghost" onclick={() => (open = false)}>{m['creator.cancel']()}</button>
		<button type="button" class="primary" onclick={start} disabled={twice !== null || !rows.length} data-testid="slice-import-start">
			{m['creator.import_slices_start']({ replace: replacing, add: adding })}
		</button>
	{/snippet}
</Sheet>

<style>
	/* the same inset and button as the tree's "Add group" (owner 2026-10-06, BACKLOG M.11) */
	.slice-import {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding: var(--space-1);
	}
	/* one line in the narrow sidebar, the full text in the tooltip */
	.slice-import > button {
		display: block;
		min-width: 0;
		padding-inline: var(--space-05);
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.rows {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}
	.rows li {
		display: grid;
		grid-template-columns: 2.125rem minmax(0, 1fr) minmax(0, 14rem);
		gap: var(--space-05);
		align-items: center;
		padding: var(--space-025) 0;
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	.rows img {
		width: 2.125rem;
		height: 3.375rem;
		object-fit: cover;
		background: var(--surface-panel);
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.where {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin-top: var(--space-1);
	}
	.where select {
		width: auto;
	}
</style>
