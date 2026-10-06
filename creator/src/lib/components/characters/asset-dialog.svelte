<script lang="ts">
	import { Badge, Sheet } from '@scutoons/ui';
	import type { AssetRequest } from '@scutoons/content';
	import { imageExists } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A missing (or broken) image cell: set its path – prefilled with the convention path, checked live against the CDN –
	 * or leave a note for the image supplier (an asset request in the wanted images).
	 */
	let {
		open = $bindable(false),
		title,
		path,
		convention,
		request,
		onset,
		onrequest
	}: {
		open?: boolean;
		/** e.g. "Katzenpapst · Mensch · default · happy (portrait)" */
		title: string;
		/** the stored path ('' = none) */
		path: string;
		convention: string;
		/** the open request of the cell */
		request?: AssetRequest;
		onset: (path: string) => void;
		onrequest: (note: string) => void;
	} = $props();

	// svelte-ignore state_referenced_locally
	let value = $state(path || convention);
	// svelte-ignore state_referenced_locally
	let note = $state(request?.note ?? '');
	let status = $state<'checking' | 'found' | 'missing' | 'empty'>('checking');

	$effect(() => {
		const p = value.trim();
		if (!p) {
			status = 'empty';
			return;
		}
		status = 'checking';
		let current = true;
		const timer = setTimeout(() => imageExists(p).then((ok) => current && (status = ok ? 'found' : 'missing')), 250);
		return () => {
			current = false;
			clearTimeout(timer);
		};
	});

	function save() {
		onset(value.trim());
		open = false;
	}
	function ask() {
		onrequest(note);
		open = false;
	}
</script>

<Sheet bind:open {title} closeLabel={m['creator.close']()}>
	<div class="asset" data-testid="asset-dialog">
		<section>
			<h3>{m['creator.characters_set_path']()}</h3>
			<div class="row">
				<div class="thumb">{#if status === 'found'}<img src={asset(value.trim())} alt="" />{/if}</div>
				<div class="col">
					<input type="text" bind:value data-testid="asset-path" />
					<span class="state" class:bad={status === 'missing'} data-testid="asset-status">
						{#if status === 'found'}<Badge tone="published">{m['creator.characters_found']()}</Badge>{:else if status === 'missing'}<Badge tone="error">{m['creator.characters_missing_file']()}</Badge>{:else if status === 'checking'}…{/if}
					</span>
					{#if value.trim() !== convention}
						<button type="button" class="ghost mini" onclick={() => (value = convention)}>↺ {m['creator.characters_use_convention_one']()}</button>
					{/if}
				</div>
			</div>
			<button type="button" onclick={save} disabled={!value.trim()} data-testid="asset-save-path">{m['creator.characters_save_path']()}</button>
			{#if status === 'missing'}<p class="hint">{m['creator.characters_path_missing_hint']()}</p>{/if}
		</section>
		<section>
			<h3>{m['creator.characters_leave_note']()}</h3>
			{#if request}
				<p class="hint">{m['creator.characters_requested']({ at: new Date(request.requested_at).toLocaleString(), by: request.requested_by ?? '–' })}</p>
			{/if}
			<textarea rows="3" bind:value={note} placeholder={m['creator.characters_note_placeholder']()} data-testid="asset-note"></textarea>
			<p class="hint">{m['creator.characters_note_hint']({ file: convention.split('/').pop() ?? convention })}</p>
			<button type="button" class="secondary" onclick={ask} data-testid="asset-request">{request ? m['creator.characters_update_request']() : m['creator.characters_request']()}</button>
		</section>
	</div>
</Sheet>

<style>
	.asset {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.25rem;
		min-width: min(44rem, 88vw);
	}
	@media (max-width: 700px) {
		.asset {
			grid-template-columns: 1fr;
		}
	}
	section {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		align-items: flex-start;
	}
	h3 {
		margin: 0;
	}
	.row {
		display: flex;
		gap: 0.5rem;
		width: 100%;
	}
	.col {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 1;
		min-width: 0;
	}
	.thumb {
		width: 5rem;
		height: 5rem;
		flex: none;
		background: var(--surface-hover);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.thumb img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.state.bad {
		color: var(--error);
	}
	textarea {
		width: 100%;
	}
	.mini {
		font-size: var(--type-caption-size);
		align-self: flex-start;
	}
</style>
