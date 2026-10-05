<script lang="ts">
	import { aposeConventionPath, bodySlots, pickText, type AposeLook } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { MediaSlot } from '@scutoons/media';

	/**
	 * The character's A-pose looks (the reader's avatar body, character level) as image slots (owner 2026-10-03: "the
	 * standard A-pose is a case for the asset picker"): the standard look (swim pants) is always there, an empty
	 * slot until a file is picked or uploaded (the picker suggests the character's folder; the convention name is the
	 * tooltip) – picking creates the look in the registry, × clears it (a retired look – the removed naked one – is
	 * removed with it; the validation warns about it). The wear slots (slot instances: id, type, label,
	 * anchor x/y/size in image px – a duo has a type twice) are read-only numbers in a collapsed technical section (they
	 * come from the characters import). No anchor dragging – outfitting happens in the reader's inventory.
	 */
	let {
		character,
		looks,
		locale,
		bust = 0,
		onchange
	}: {
		character: string;
		/** the stored looks plus the standard ones not stored yet */
		looks: AposeLook[];
		locale: string;
		/** set after an import (a timestamp): previews load again */
		bust?: number;
		/** a look's file picked (a CDN-root path) or cleared (undefined) */
		onchange: (look: string, path: string | undefined) => void;
	} = $props();
</script>

<section class="apose" data-testid="apose-editor">
	<div class="looks">
		{#each looks as look (look.id)}
			{@const label = pickText(look.label, locale, ['en', 'de']) || look.id}
			<article class="look" data-testid="apose-{look.id}" title={look.path || aposeConventionPath(character, look.id)}>
				<MediaSlot
					toon=""
					value={look.path?.trim() || undefined}
					label={look.default ? `${label} · ${m['creator.characters_default']()}` : label}
					place="characters/{character}/"
					{bust}
					onchange={(path) => onchange(look.id, path)}
					testid="apose-{look.id}-slot"
				/>
				{#if look.path && look.slots?.length}
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
				{/if}
			</article>
		{/each}
	</div>
</section>

<style>
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
</style>
