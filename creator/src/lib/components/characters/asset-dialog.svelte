<script lang="ts">
	import { Badge, Icon, Sheet } from '@scutoons/ui';
	import { imageExists } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A missing (or broken) image cell: set its path – prefilled with the convention path, checked live against the CDN – or
	 * pick the file in the Library. No wanted-image note (RULES H17: the tools do not flag what is missing).
	 */
	let {
		open = $bindable(false),
		title,
		path,
		convention,
		onset,
		onlibrary
	}: {
		open?: boolean;
		/** e.g. "Katzenpapst · Mensch · default · happy (portrait)" */
		title: string;
		/** the stored path ('' = none) */
		path: string;
		convention: string;
		onset: (path: string) => void;
		/** pick the file in the Library instead (the slot on top, only what fits) */
		onlibrary?: () => void;
	} = $props();

	// svelte-ignore state_referenced_locally
	let value = $state(path || convention);
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
			<span class="buttons">
				<button type="button" onclick={save} disabled={!value.trim()} data-testid="asset-save-path">{m['creator.characters_save_path']()}</button>
				{#if onlibrary}<button type="button" class="secondary" onclick={() => ((open = false), onlibrary())} data-testid="asset-library"><Icon name="grid" size="0.9rem" /> {m['library.title']()}</button>{/if}
			</span>
			{#if status === 'missing'}<p class="hint">{m['creator.characters_path_missing_hint']()}</p>{/if}
		</section>
	</div>
</Sheet>

<style>
	.asset {
		display: grid;
		gap: 1.25rem;
		width: min(30rem, 88vw);
	}
	.buttons {
		display: flex;
		gap: 0.4rem;
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
		background: var(--tool-hover);
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
		color: var(--tool-stop);
	}
	.mini {
		font-size: var(--type-caption-size);
		align-self: flex-start;
	}
</style>
