<script lang="ts">
	import { goto } from '$app/navigation';
	import type { AssetUsage } from '@scutoons/api';
	import { bytes, formatDuration, type AssetFamily } from '@scutoons/media';
	import { Section } from '@scutoons/ui/editor';
	import { library } from '$lib/editor/library.svelte.ts';
	import { hrefOf } from '$lib/shell/address.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The browser's details drawer: the preview, the versions table (format, size, quality, the expected ones missing),
	 * where the family is used (a click opens that item), the actions. The uses are the registry's (content + catalog);
	 * credits per asset: TODO(#690).
	 */
	let { family, onreplace }: { family: AssetFamily; onreplace: (file: File) => void } = $props();

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

	/** one line per item that uses it (the fields it uses it in, joined) */
	const uses = $derived.by(() => {
		const byRef = new Map<string, { ref: string; fields: Set<string>; state: string; href: string | null }>();
		for (const u of usages ?? []) {
			const row = byRef.get(u.ref) ?? { ref: u.ref, fields: new Set<string>(), state: u.state, href: u.source === 'code' ? null : hrefOf(u.ref) };
			row.fields.add(u.field.split('/').filter(Boolean).at(-1) ?? u.field);
			byRef.set(u.ref, row);
		}
		return [...byRef.values()];
	});
	const video = $derived(family.versions.find((v) => v.kind === 'video' && v.label === '480p') ?? family.versions.find((v) => v.kind === 'video'));
	const audio = $derived(family.versions.find((v) => v.kind === 'audio'));
	const dims = (w: number | null, h: number | null) => (w && h ? `${w} × ${h}` : '');

	function jump(href: string) {
		library.browser = false;
		goto(href);
	}
	function copy() {
		navigator.clipboard?.writeText(family.main).catch(() => {});
	}
</script>

<aside class="details" data-testid="library-details">
	<div class="preview">
		{#if family.kind === 'video' && video}
			<!-- svelte-ignore a11y_media_has_caption -->
			<video src={asset(video.path)} poster={family.preview ? asset(family.preview) : undefined} controls preload="metadata"></video>
		{:else if family.kind === 'audio' && audio}
			<audio src={asset(audio.path)} controls preload="none"></audio>
		{:else if family.preview}
			<img src={asset(family.preview)} alt="" />
		{/if}
	</div>
	<h2>{family.title}</h2>
	<p class="mono sub">{family.key} · {m[`library.kind_${family.kind}`]()} · {m['library.versions_count']({ count: family.versions.length })}</p>

	<Section id="library-details-versions" label={m['library.versions']()} summary={m['library.versions_summary']({ ready: family.versions.length, missing: family.missing.length })}>
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
				{#each family.missing as label (label)}
					<tr class="missing"><td class="mono">{label}</td><td class="mono" colspan="3">{m['library.missing']()}</td></tr>
				{/each}
			</tbody>
		</table>
	</Section>

	<Section id="library-details-used" label={m['library.where_used']()} summary={usages ? m['library.uses']({ count: uses.length }) : ''}>
		{#if failed}
			<p class="sc-empty">{m['library.detail_failed']()}</p>
		{:else if usages && uses.length}
			<ul class="uses">
				{#each uses as u (u.ref)}
					<li>
						{#if u.href}<button type="button" class="jump" data-control-exempt="text link" onclick={() => jump(u.href!)}>{u.ref}</button>{:else}<span class="mono">{u.ref}</span>{/if}
						<span class="mono muted">{[...u.fields].join(', ')}</span>
						{#if u.state === 'draft'}<span class="tag">{m['library.draft']()}</span>{/if}
					</li>
				{/each}
			</ul>
		{:else if usages}
			<p class="sc-empty">{m['library.unused_hint']()}</p>
		{/if}
	</Section>

	<Section id="library-details-actions" label={m['library.actions']()}>
		<div class="actions">
			<button type="button" class="primary" onclick={() => library.use(family)} disabled={!library.usable(family)}>{m['library.use']()}</button>
			<button type="button" onclick={() => fileInput?.click()}>{m['library.replace']()}</button>
			<button type="button" onclick={copy}>{m['library.copy_path']()}</button>
			<a class="sc-button" href={asset(family.main)} target="_blank" rel="noopener">{m['library.open_file']()}</a>
		</div>
		<input bind:this={fileInput} type="file" hidden onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) onreplace(f); e.currentTarget.value = ''; }} />
	</Section>
</aside>

<style>
	.details {
		overflow: auto;
		padding: var(--editor-pad);
		border-left: var(--editor-edge) solid var(--editor-line);
		font-size: var(--editor-small);
	}
	.preview {
		display: grid;
		place-items: center;
		aspect-ratio: 16 / 10;
		border-radius: var(--editor-radius);
		background: var(--editor-stage);
		overflow: hidden;
	}
	.preview img,
	.preview video {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}
	.preview audio {
		width: 90%;
	}
	h2 {
		margin: 0.6rem 0 0.1rem;
		font-size: var(--editor-title, 1.15rem);
		font-weight: 400;
		letter-spacing: normal;
		text-transform: none;
	}
	.sub {
		margin: 0 0 0.5rem;
		color: var(--editor-muted);
		word-break: break-all;
	}
	.mono {
		font-family: var(--editor-mono, ui-monospace, monospace);
	}
	.muted {
		color: var(--editor-muted);
	}
	.versions {
		width: 100%;
		border-collapse: collapse;
	}
	.versions td {
		padding: 0.25rem 0.3rem 0.25rem 0;
		border-bottom: var(--editor-edge) solid var(--editor-line);
		white-space: nowrap;
	}
	.versions .num {
		text-align: right;
	}
	.versions .missing {
		color: var(--editor-warn);
	}
	.tag {
		padding: 0 0.35rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1rem;
		color: var(--editor-muted);
		font-size: var(--editor-label);
	}
	.uses {
		margin: 0;
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
		color: var(--editor-accent);
		font: inherit;
		cursor: pointer;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}
	.actions .primary {
		border-color: var(--editor-primary-line);
		background: var(--editor-primary);
		color: var(--editor-primary-ink);
	}
</style>
