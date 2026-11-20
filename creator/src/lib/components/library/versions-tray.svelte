<script lang="ts">
	import { bytes, type AssetFamily, type FamilyVersion } from '@scutoons/media';
	import { Icon } from '@scutoons/ui';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A family's versions as a tray under its tile (the ⧉ badge): every file with its label and format, the expected ones
	 * it lacks dashed. A click uses exactly that file.
	 */
	let { family, onuse, onclose }: { family: AssetFamily; onuse: (version: FamilyVersion) => void; onclose: () => void } = $props();

	const ICONS: Record<FamilyVersion['kind'], string> = { image: 'image', video: 'film', audio: 'music', model: 'box', other: 'file' };
	const total = $derived(family.versions.reduce((n, v) => n + v.size, 0));
</script>

<div class="tray" data-testid="versions-tray">
	<div class="head">
		<span class="title">{family.title}</span>
		<span class="sub">{m['library.tray_sub']({ count: family.versions.length, size: bytes(total) })}</span>
		<button type="button" class="close" data-control-exempt="text link" onclick={onclose}>{m['library.close']()}</button>
	</div>
	<div class="files">
		{#each family.versions as v (v.path)}
			<button type="button" class="v" data-control-exempt="version tile" title={v.path} onclick={() => onuse(v)}>
				{#if v.kind === 'image'}<img src={asset(v.path)} alt="" loading="lazy" />{:else}<span class="glyph"><Icon name={ICONS[v.kind]} size="1.2rem" /></span>{/if}
				<span class="label mono">{v.label} {v.ext}</span>
			</button>
		{/each}
		{#each family.missing as label (label)}
			<span class="v missing" title={m['library.missing_version']({ label })}><span class="label mono">{label}</span><span class="mono">{m['library.missing']()}</span></span>
		{/each}
	</div>
</div>

<style>
	.tray {
		grid-column: 1 / -1;
		border: var(--editor-edge) solid var(--editor-accent);
		border-radius: var(--editor-radius);
		padding: 0.35rem;
	}
	.head {
		display: flex;
		align-items: baseline;
		gap: 0.4rem;
		margin-bottom: 0.35rem;
		font-size: var(--editor-small);
	}
	.sub {
		flex: 1 1 auto;
		color: var(--editor-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.close {
		border: 0;
		background: none;
		color: var(--editor-accent);
		cursor: pointer;
		font: inherit;
	}
	.files {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(4.2rem, 1fr));
		gap: 0.3rem;
	}
	.v {
		position: relative;
		aspect-ratio: 1;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-stage);
		overflow: hidden;
		padding: 0;
		cursor: pointer;
		display: grid;
		place-items: center;
		color: var(--editor-muted);
	}
	.v img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.v .label {
		position: absolute;
		left: 0.15rem;
		bottom: 0.15rem;
		padding: 0 0.25rem;
		border-radius: 0.25rem;
		background: var(--editor-surface);
		color: var(--editor-text);
		font-size: var(--editor-label);
	}
	.v.missing {
		border-style: dashed;
		border-color: var(--editor-warn);
		color: var(--editor-warn);
		cursor: default;
		font-size: var(--editor-label);
		text-transform: uppercase;
	}
	.v.missing .label {
		position: static;
		background: none;
		color: inherit;
	}
	.mono {
		font-family: var(--editor-mono, ui-monospace, monospace);
	}
</style>
