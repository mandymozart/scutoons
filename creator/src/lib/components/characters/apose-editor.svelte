<script lang="ts">
	import {
		APOSE_VIEW,
		AVATAR_SLOTS,
		aposeConventionPath,
		pickText,
		type AposeLook,
		type AssetRequest,
		type AvatarSlot
	} from '@scutoons/content';
	import { imageExists } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AssetDialog from './asset-dialog.svelte';

	/**
	 * The character's A-pose looks (the reader's avatar body, character level): read-only thumbnails with a path field
	 * checked against the CDN, the wear anchors as plain numbers in a collapsed technical section, and the usual
	 * "missing – add image" / asset request for a look without an image. No look creator and no anchor dragging –
	 * outfitting happens in the reader's inventory.
	 */
	let {
		character,
		looks,
		requests,
		locale,
		onchange,
		onrequest
	}: {
		character: string;
		looks: AposeLook[];
		/** open requests of this character (view apose) */
		requests: AssetRequest[];
		locale: string;
		/** one A-pose look changed */
		onchange: (look: string, next: AposeLook, key?: string) => void;
		onrequest: (look: string, note: string) => void;
	} = $props();

	let found = $state<Record<string, boolean | undefined>>({});
	let dialogLook = $state<string | null>(null);
	let dialogOpen = $state(false);

	$effect(() => {
		for (const l of looks) {
			const path = l.path?.trim();
			if (path && !(path in found)) {
				found[path] = undefined;
				imageExists(path).then((ok) => (found[path] = ok));
			}
		}
	});

	function setAnchor(look: AposeLook, slot: AvatarSlot, key: 'x' | 'y' | 'size', raw: string) {
		const value = Math.max(0, Math.round(Number(raw) || 0));
		const anchors = { ...(look.anchors ?? {}) };
		anchors[slot] = { ...(anchors[slot] ?? { x: 0, y: 0, size: 0 }), [key]: value };
		onchange(look.id, { ...look, anchors }, `apose-${look.id}-${slot}-${key}`);
	}
	const requestOf = (id: string) => requests.find((r) => r.view === APOSE_VIEW && r.look === id && r.status === 'open');
	const dialog = $derived(looks.find((l) => l.id === dialogLook));
</script>

<section class="apose" data-testid="apose-editor">
	<div class="looks">
		{#each looks as look (look.id)}
			{@const path = look.path?.trim() ?? ''}
			{@const state = path ? found[path] : undefined}
			{@const request = requestOf(look.id)}
			<article class="look" data-testid="apose-{look.id}">
				<header>
					<strong>{pickText(look.label, locale, ['en', 'de']) || look.id}</strong>
					{#if look.default}<span class="hint">{m['creator.characters_default']()}</span>{/if}
				</header>
				{#if path}
					<div class="thumb" class:bad={state === false}>
						{#if state === true}<img src={asset(path)} alt="A-pose {look.id}" loading="lazy" />{:else if state === false}<span>✗ {m['creator.characters_missing_file']()}</span>{:else}…{/if}
					</div>
					<label>
						{m['creator.characters_apose_path']()}
						<input type="text" value={path} onchange={(e) => onchange(look.id, { ...look, path: e.currentTarget.value.trim() }, `apose-${look.id}-path`)} data-testid="apose-{look.id}-path" />
						<span class="state" class:bad={state === false} data-testid="apose-{look.id}-state">{state === true ? `✓ ${m['creator.characters_found']()}` : state === false ? `✗ ${m['creator.characters_missing_file']()}` : '…'}</span>
					</label>
					<details class="tech">
						<summary>{m['creator.characters_apose_anchors']()}</summary>
						<table>
							<thead><tr><th>{m['creator.characters_slot']()}</th><th>x</th><th>y</th><th>{m['creator.characters_size']()}</th></tr></thead>
							<tbody>
								{#each AVATAR_SLOTS as slot (slot)}
									{@const a = look.anchors?.[slot]}
									<tr>
										<td>{slot}</td>
										{#each ['x', 'y', 'size'] as const as key (key)}
											<td><input type="number" min="0" value={a?.[key] ?? ''} onchange={(e) => setAnchor(look, slot, key, e.currentTarget.value)} data-testid="anchor-{look.id}-{slot}-{key}" /></td>
										{/each}
									</tr>
								{/each}
							</tbody>
						</table>
					</details>
				{:else}
					<button type="button" class="missing" onclick={() => ((dialogLook = look.id), (dialogOpen = true))} data-testid="apose-{look.id}-missing">
						<span>{m['creator.characters_missing_add']()}</span>
						<code>{aposeConventionPath(character, look.id).split('/').pop()}</code>
						{#if request}<span class="req" title={request.requested_by ?? ''}>✉ {request.note || m['creator.characters_requested_short']()}</span>{/if}
					</button>
				{/if}
			</article>
		{:else}
			<p class="hint">{m['creator.characters_apose_none']()}</p>
		{/each}
	</div>
</section>

{#if dialog}
	{#key dialog.id}
		<AssetDialog
			bind:open={dialogOpen}
			title="{character} · A-pose · {pickText(dialog.label, locale, ['en', 'de']) || dialog.id}"
			path=""
			convention={aposeConventionPath(character, dialog.id)}
			request={requestOf(dialog.id)}
			onset={(path) => onchange(dialog.id, { ...dialog, path }, `apose-${dialog.id}-path`)}
			onrequest={(note) => onrequest(dialog.id, note)}
		/>
	{/key}
{/if}

<style>
	.apose {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.looks {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
	}
	.look {
		width: 14rem;
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		font-size: 12px;
	}
	header {
		display: flex;
		gap: 0.4rem;
		align-items: baseline;
	}
	.thumb {
		aspect-ratio: 2 / 3;
		width: 100%;
		border: 1px solid var(--ink);
		background: var(--surface-hover);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.thumb.bad {
		border-color: var(--error);
		color: var(--error);
	}
	.thumb img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	label {
		font-size: 11px;
		font-weight: 700;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.state {
		font-weight: 400;
	}
	.bad {
		color: var(--error);
	}
	.tech summary {
		cursor: pointer;
		font-size: 11px;
	}
	table {
		border-collapse: collapse;
		font-size: 11px;
		margin-top: 0.3rem;
	}
	th {
		text-align: left;
		padding: 0.1rem 0.2rem;
	}
	td {
		padding: 0.1rem 0.15rem;
	}
	td input {
		width: 3.8rem;
	}
	.missing {
		all: unset;
		cursor: pointer;
		box-sizing: border-box;
		border: 1px dashed var(--ink);
		width: 100%;
		aspect-ratio: 2 / 3;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.3rem;
		font-size: 11px;
		text-align: center;
		padding: 0.5rem;
	}
	.missing:focus-visible {
		outline: 2px solid var(--ink);
	}
	.req {
		background: var(--primary-cyan);
		padding: 0 0.25rem;
		font-size: 10px;
	}
</style>
