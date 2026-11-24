<script lang="ts">
	import { goto } from '$app/navigation';
	import type { AssetUsage } from '@scutoons/api';
	import { bytes, formatDuration, previewPicture, type AssetFamily } from '@scutoons/media';
	import { CloseButton, Icon } from '@scutoons/ui';
	import { Section } from '@scutoons/ui/editor';
	import { library } from '$lib/editor/library.svelte.ts';
	import { copyPath, deleteFamilies, downloadFamilies, renameFamily } from '$lib/editor/library-actions.ts';
	import { hrefOf } from '$lib/shell/address.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import VersionsTray from '$lib/components/library/versions-tray.svelte';

	/**
	 * A family's details – the browser's drawer at the right, the panel's at its bottom (`drawer`): the preview, the
	 * name (F2 renames it: URL and uses stay), Use / Set as cover, where the open item uses it (each part a jump to it),
	 * where else the registry knows it used, its versions (a tray to use one exactly, the table),
	 * Download, Copy path, Credits (#690), Delete.
	 */
	let { family, drawer = false, onreplace, onclose }: { family: AssetFamily; drawer?: boolean; onreplace: (file: File) => void; onclose?: () => void } = $props();

	let usages = $state<AssetUsage[] | null>(null);
	let failed = $state(false);
	let fileInput: HTMLInputElement | null = $state(null);

	$effect(() => {
		const f = family;
		usages = null;
		failed = false;
		let current = true;
		Promise.all(f.assets.map((id) => library.detail(id)))
			.then((details) => current && (usages = details.flatMap((d) => d.usages)))
			.catch(() => current && (failed = true));
		return () => (current = false);
	});

	/** the open item's uses, each part a jump to it */
	const parts = $derived(library.usesOf(family.key));
	/** one line per other item the registry names (the fields it uses it in, joined) */
	const others = $derived.by(() => {
		const byRef = new Map<string, { ref: string; fields: Set<string>; state: string; href: string | null }>();
		for (const u of usages ?? []) {
			if (library.address && (u.ref === library.address || u.ref.startsWith(`${library.address}:`))) continue;
			const row = byRef.get(u.ref) ?? { ref: u.ref, fields: new Set<string>(), state: u.state, href: u.source === 'code' ? null : hrefOf(u.ref) };
			row.fields.add(u.field.split('/').filter(Boolean).at(-1) ?? u.field);
			byRef.set(u.ref, row);
		}
		return [...byRef.values()];
	});
	const fits = $derived(!library.pick || library.takes(family.main, library.pick).ok);
	const chapterOpen = $derived(!!library.address?.startsWith('chapter:') && family.kind === 'image');
	const picture = $derived(previewPicture(family.preview));
	const video = $derived(family.versions.find((v) => v.kind === 'video' && v.label === '480p') ?? family.versions.find((v) => v.kind === 'video'));
	const audio = $derived(family.versions.find((v) => v.kind === 'audio'));
	const dims = (w: number | null, h: number | null) => (w && h ? `${w} × ${h}` : '');

	function jump(href: string) {
		goto(href);
	}
	const at = (item: string, anchor: string) => {
		const href = hrefOf(item);
		return href && anchor ? `${href}${href.includes('?') ? '&' : '?'}${anchor}` : href;
	};
	function setCover() {
		window.dispatchEvent(new CustomEvent('sc-library-set', { detail: { role: 'thumbnail', path: family.main, item: library.address } }));
	}
</script>

<aside class="details" class:drawer data-testid="library-details">
	{#if drawer && onclose}<span class="close"><CloseButton label={m['library.close']()} onclick={onclose} /></span>{/if}
	<div class="preview">
		{#if family.kind === 'video' && video}
			<!-- svelte-ignore a11y_media_has_caption -->
			<video src={asset(video.path)} poster={picture ? asset(picture) : undefined} controls preload="metadata"></video>
		{:else if family.kind === 'audio' && audio}
			<audio src={asset(audio.path)} controls preload="none"></audio>
		{:else if picture}
			<img src={asset(picture)} alt="" />
		{/if}
	</div>
	<h2><button type="button" class="name" data-control-exempt="text link" title={m['library.rename']()} onclick={() => renameFamily(family)}>{family.title}</button></h2>
	<p class="mono sub">{family.key} · {m[`library.kind_${family.kind}`]()}{#if dims(family.width, family.height)} · {dims(family.width, family.height)}{/if} · {bytes(family.size)}{#if family.version > 1} · v{family.version}{/if}</p>

	<!-- what can be done with it here (#750): Use only while a slot picks, the cover only in a chapter; the rest as icons -->
	<div class="actions">
		{#if library.pick}<button type="button" class="primary" onclick={() => library.use(family)} disabled={!fits}>{m['library.use_here']()}</button>{/if}
		{#if chapterOpen && !library.pick}<button type="button" onclick={setCover} data-testid="library-set-cover">{m['library.set_cover']()}</button>{/if}
		<span class="icons">
			<button type="button" class="ghost icon" title={m['library.download']()} aria-label={m['library.download']()} onclick={() => downloadFamilies([family])}><Icon name="download" size="1rem" /></button>
			<button type="button" class="ghost icon" title={m['library.copy_path']()} aria-label={m['library.copy_path']()} onclick={() => copyPath(family)}><Icon name="copy" size="1rem" /></button>
			<button type="button" class="ghost icon" title={m['library.replace']()} aria-label={m['library.replace']()} onclick={() => fileInput?.click()}><Icon name="upload" size="1rem" /></button>
			<button type="button" class="ghost icon danger" title={m['library.delete']()} aria-label={m['library.delete']()} onclick={() => deleteFamilies([family])}><Icon name="trash-2" size="1rem" /></button>
		</span>
	</div>
	<input bind:this={fileInput} type="file" hidden onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) onreplace(f); e.currentTarget.value = ''; }} />

	<Section id="library-details-used" label={m['library.where_used']()} summary={String(parts.length + others.length)}>
		{#if parts.length}
			<ul class="uses" data-testid="library-uses">
				{#each parts as u (u.part + u.path)}
					{@const href = at(u.item, u.at)}
					<li>{#if href}<button type="button" class="jump" data-control-exempt="text link" onclick={() => jump(href)}>{u.part}</button>{:else}<span>{u.part}</span>{/if}</li>
				{/each}
			</ul>
		{/if}
		{#if failed}
			<p class="sc-empty">{m['library.detail_failed']()}</p>
		{:else if others.length}
			<ul class="uses">
				{#each others as u (u.ref)}
					<li>
						{#if u.href}<button type="button" class="jump" data-control-exempt="text link" onclick={() => jump(u.href!)}>{u.ref}</button>{:else}<span class="mono">{u.ref}</span>{/if}
						<span class="mono muted">{[...u.fields].join(', ')}</span>
						{#if u.state === 'draft'}<span class="tag">{m['library.draft']()}</span>{/if}
					</li>
				{/each}
			</ul>
		{:else if usages && !parts.length}
			<p class="sc-empty">{m['library.unused_hint']()}</p>
		{/if}
	</Section>

	<Section id="library-details-versions" label={m['library.versions']()} open={!drawer} summary={m['library.versions_count']({ count: family.versions.length })}>
		{#if family.versions.length > 1}<VersionsTray {family} onuse={(v) => library.use(family, v.path)} />{/if}
		<table class="versions">
			<tbody>
				{#each family.versions as v (v.path)}
					<tr title={v.path}>
						<td class="mono">{v.label}</td>
						<td class="mono muted">{v.ext}{#if dims(v.width, v.height)} · {dims(v.width, v.height)}{/if}{#if v.duration} · {formatDuration(v.duration)}{/if}</td>
						<td class="mono num">{bytes(v.size)}</td>
						<td>{#if v.quality}<span class="tag">{v.quality}</span>{/if}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</Section>
</aside>

<style>
	.details {
		position: relative;
		overflow: auto;
		padding: var(--editor-pad);
		border-left: var(--editor-edge) solid var(--tool-line);
		font-size: var(--editor-small);
	}
	.details.drawer {
		position: sticky;
		bottom: 0;
		z-index: 1;
		/* the panel's body has no set height, so a % would never bite: the drawer keeps the grid two rows at least */
		max-height: min(40vh, 22rem);
		margin: 0 calc(-1 * var(--editor-pad));
		border-left: 0;
		border-top: var(--editor-edge) solid var(--tool-line);
		background: var(--tool-popover);
		text-shadow: var(--tool-halo);
	}
	.close {
		position: absolute;
		top: 0.3rem;
		right: 0.3rem;
		z-index: 1;
	}
	.preview {
		position: relative;
		display: grid;
		place-items: center;
		aspect-ratio: 16 / 10;
		border-radius: var(--editor-radius);
		background: var(--tool-ground);
		overflow: hidden;
	}
	.drawer .preview {
		aspect-ratio: auto;
		height: 7rem;
	}
	/* out of the grid's flow: a % height there doesn't resolve and a tall picture showed only its top strip */
	.preview img,
	.preview video {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.preview audio {
		width: 90%;
	}
	h2 {
		margin: 0.6rem 0 0.1rem;
		font-family: var(--editor-sans);
		font-size: var(--editor-title, 1.15rem);
		font-weight: 400;
	}
	.name {
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: start;
		cursor: text;
	}
	.sub {
		margin: 0 0 0.5rem;
		color: var(--tool-muted);
		word-break: break-all;
	}
	.mono {
		font-family: var(--editor-mono, ui-monospace, monospace);
	}
	.muted {
		color: var(--tool-muted);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.3rem;
		margin-bottom: 0.4rem;
	}
	/* the icon actions sit at the row's end; delete is red only under the pointer */
	.icons {
		display: flex;
		gap: 0.1rem;
		margin-left: auto;
	}
	.icon {
		width: var(--editor-row);
		padding: 0;
		color: var(--tool-muted);
	}
	.icon:hover {
		color: var(--tool-text);
	}
	.icon.danger:hover {
		color: var(--tool-danger);
	}
	.versions {
		width: 100%;
		margin-top: 0.4rem;
		border-collapse: collapse;
	}
	.versions td {
		padding: 0.25rem 0.3rem 0.25rem 0;
		border-bottom: var(--editor-edge) solid var(--tool-line);
		white-space: nowrap;
	}
	.versions .num {
		text-align: right;
	}
	.tag {
		padding: 0 0.35rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 1rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
	}
	.uses {
		margin: 0 0 0.3rem;
		padding: 0;
		list-style: none;
		display: grid;
		gap: 0.3rem;
	}
	.uses li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.4rem;
	}
	.jump {
		padding: 0;
		border: 0;
		background: none;
		color: var(--tool-text);
		font: inherit;
		text-align: start;
		cursor: pointer;
	}
</style>
