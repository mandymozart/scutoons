<script lang="ts">
	import { Badge, Icon } from '@scutoons/ui';
	import {
		APOSE_VIEW,
		aposeConventionPath,
		bodySlots,
		pickText,
		type AposeLook,
		type AssetRequest
	} from '@scutoons/content';
	import { imageExists } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AssetDialog from '$lib/components/characters/asset-dialog.svelte';

	/**
	 * The character's A-pose looks (the reader's avatar body, character level): read-only thumbnails with a path field
	 * checked against the CDN, the wear slots (slot instances: id, type, label, anchor x/y/size in image px – a duo has
	 * a type twice) as read-only numbers in a collapsed technical section (they come from the characters import), and the
	 * usual "missing – add image" / asset request for a look without an image. No look creator and no anchor dragging –
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
					<span>{pickText(look.label, locale, ['en', 'de']) || look.id}</span>
					{#if look.default}<span class="hint">{m['creator.characters_default']()}</span>{/if}
				</header>
				{#if path}
					<div class="thumb" class:bad={state === false}>
						{#if state === true}<img src={asset(path)} alt="A-pose {look.id}" loading="lazy" />{:else if state === false}<Badge tone="error">{m['creator.characters_missing_file']()}</Badge>{:else}…{/if}
					</div>
					<label>
						{m['creator.characters_apose_path']()}
						<input type="text" value={path} onchange={(e) => onchange(look.id, { ...look, path: e.currentTarget.value.trim() }, `apose-${look.id}-path`)} data-testid="apose-{look.id}-path" />
						<span class="state" data-testid="apose-{look.id}-state">{#if state === true}<Badge tone="published">{m['creator.characters_found']()}</Badge>{:else if state === false}<Badge tone="error">{m['creator.characters_missing_file']()}</Badge>{:else}…{/if}</span>
					</label>
					<details class="tech">
						<summary>{m['creator.characters_apose_anchors']()}</summary>
						<table>
							<thead><tr><th>id</th><th>{m['creator.characters_slot']()}</th><th>x</th><th>y</th><th>{m['creator.characters_size']()}</th></tr></thead>
							<tbody>
								{#each bodySlots(look.slots) as slot (slot.id)}
									<tr data-testid="slot-{look.id}-{slot.id}" title={pickText(slot.label, locale, ['en', 'de'])}>
										<td>{slot.id}</td>
										<td>{slot.slot}</td>
										<td class="num">{slot.x}</td>
										<td class="num">{slot.y}</td>
										<td class="num">{slot.size || '–'}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</details>
				{:else}
					<button type="button" class="missing" data-control-exempt="missing picture card" onclick={() => ((dialogLook = look.id), (dialogOpen = true))} data-testid="apose-{look.id}-missing">
						<span>{m['creator.characters_missing_add']()}</span>
						<code>{aposeConventionPath(character, look.id).split('/').pop()}</code>
						{#if request}<span class="sc-mark" title={request.requested_by ?? ''}><Icon name="mail" /> {request.note || m['creator.characters_requested_short']()}</span>{/if}
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
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		font-weight: var(--font-weight-regular);
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
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	table {
		border-collapse: collapse;
		margin-top: 0.3rem;
	}
	th {
		text-align: left;
		padding: 0.1rem 0.2rem;
	}
	td {
		padding: 0.1rem 0.15rem;
	}
	td.num {
		text-align: right;
		font-variant-numeric: tabular-nums;
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
		font-size: var(--type-caption-size);
		text-align: center;
		padding: 0.5rem;
	}
	.missing:focus-visible {
		outline: 2px solid var(--ink);
	}
</style>
