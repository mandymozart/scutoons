<script lang="ts">
	import { CloseIcon, Icon } from '@scutoons/ui';
	import type { Supporter } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SharedField from './shared-field.svelte';
	import MediaSlot from './media-slot.svelte';

	/**
	 * The season's supporters (`supporters: [{ name, logo?, url? }]`, in order), the same in every language: the reader
	 * shows them as "Supported by" logos at the end of the credits fine print (agents/DESIGN.md "Credits colophon").
	 */
	let { doc, toon, place = '' }: { doc: ContentDoc; toon: string; /** where logo uploads go (catpope/s1/supporters/) */ place?: string } = $props();

	const list = $derived<Supporter[]>(Array.isArray(doc.get(doc.primary, ['supporters'])) ? doc.get(doc.primary, ['supporters']) : []);

	function change(fn: (list: Supporter[]) => void) {
		doc.structural((file) => {
			if (!Array.isArray(file.supporters)) file.supporters = [];
			fn(file.supporters);
		}, 'supporters');
	}
	const add = () => change((l) => l.push({ name: '' }));
	const remove = (i: number) => change((l) => l.splice(i, 1));
	function move(i: number, by: number) {
		const to = i + by;
		if (to < 0 || to >= list.length) return;
		change((l) => l.splice(to, 0, ...l.splice(i, 1)));
	}
</script>

<ol class="supporters" data-testid="supporters">
	{#each list as supporter, i (i)}
		<li class="supporter" data-testid="supporter-{i}">
			<div class="fields">
				<!-- the logo as a slot (the preview is the button), name and link are never translated -->
				<MediaSlot {toon} {place} label="logo" value={supporter.logo} onchange={(v) => change((l) => { if (v) l[i].logo = v; else delete l[i].logo; })} testid="supporter-{i}-logo" />
				<div class="texts">
					<SharedField {doc} path={['supporters', i, 'name']} label="name" required splittable={false} testid="supporter-{i}-name" />
					<SharedField {doc} path={['supporters', i, 'url']} label="url" placeholder="https://…" splittable={false} testid="supporter-{i}-url" />
				</div>
			</div>
			<div class="actions">
				<button type="button" class="ghost" disabled={i === 0} onclick={() => move(i, -1)} aria-label={m['creator.move_up']()} title={m['creator.move_up']()}><Icon name="chevron-left" /></button>
				<button type="button" class="ghost" disabled={i === list.length - 1} onclick={() => move(i, 1)} aria-label={m['creator.move_down']()} title={m['creator.move_down']()}><Icon name="chevron-right" /></button>
				<button type="button" class="ghost" onclick={() => remove(i)} aria-label={m['creator.supporter_remove']({ name: supporter.name || String(i + 1) })} title={m['creator.remove']()}><CloseIcon /></button>
			</div>
		</li>
	{:else}
		<li class="hint">{m['creator.supporters_empty']()}</li>
	{/each}
</ol>
<button type="button" class="secondary" onclick={add} data-testid="supporter-add">{m['creator.supporter_add']()}</button>

<style>
	.supporters {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.supporter {
		display: flex;
		gap: 0.5rem;
		align-items: flex-start;
		padding-bottom: 0.75rem;
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	.fields {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		align-items: flex-start;
	}
	.texts {
		flex: 1 1 16rem;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.actions {
		display: flex;
		gap: 0.2rem;
		flex: none;
	}
	.actions button {
		padding: 0 0.4em;
	}
	.hint {
		color: var(--muted);
	}
	button.secondary {
		align-self: flex-start;
	}
</style>
