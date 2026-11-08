<script lang="ts">
	// The rig page of a character without a rig (#425, owner 2026-11-08: "eine figur ohne rig, sollte nicht klingen" · "das
	// ziel soll sein, sich dann dort aus bestehenden modulen ein rig zu bauen, oder neue module specifisch fuer den character
	// zu bauen"): exactly two ways in – start from existing modules (the factory modules and the creator's own, #350: one
	// module becomes the first track) or create a new module for this character (a `module:<id>` draft named after it, then
	// the Rust tab). Which characters have no rig is the core's (`rig_coverage`, the Assignment pane); what a module makes of
	// itself as a track is the core's too (`rig_module_instrument`). Wiring modules together is the patch editor's (#203).
	import { m } from '$lib/paraglide/messages.js';

	let { character, modules, factory = [], busy = false, problem = '', onstart, oncreate }: {
		character: string;
		/** the stored modules (every id); `factory` marks the factory ones */
		modules: string[];
		factory?: string[];
		busy?: boolean;
		/** the core's (or the API's) answer when the first step failed */
		problem?: string;
		onstart: (moduleId: string) => void;
		oncreate: () => void;
	} = $props();

	let picked = $state('');
</script>

<div class="empty" data-testid="rig-empty">
	<h2>{m['creator.sound_empty_title']({ character })}</h2>
	<p class="sc-empty">{m['creator.sound_empty_hint']()}</p>
	<section aria-labelledby="rig-empty-existing">
		<h3 id="rig-empty-existing">{m['creator.sound_empty_existing']()}</h3>
		<p class="hint">{m['creator.sound_empty_existing_hint']()}</p>
		<div class="row">
			<select bind:value={picked} aria-label={m['creator.sound_module']()} data-testid="rig-empty-module">
				<option value="">…</option>
				{#each modules as id (id)}<option value={id}>module:{id}{factory.includes(id) ? ` · ${m['creator.sound_factory']()}` : ''}</option>{/each}
			</select>
			<button type="button" disabled={!picked || busy} onclick={() => onstart(picked)} data-testid="rig-empty-add">{m['creator.sound_empty_add']()}</button>
		</div>
	</section>
	<section aria-labelledby="rig-empty-new">
		<h3 id="rig-empty-new">{m['creator.sound_empty_new']({ character })}</h3>
		<p class="hint">{m['creator.sound_empty_new_hint']({ id: character })}</p>
		<div class="row">
			<button type="button" disabled={busy} onclick={oncreate} data-testid="rig-empty-create">{m['creator.sound_empty_new_button']({ id: character })}</button>
		</div>
	</section>
	{#if problem}<p class="problem" role="status" data-testid="rig-empty-problem">{problem}</p>{/if}
</div>

<style>
	.empty {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-1);
		min-height: 0;
		overflow: auto;
	}
	h2,
	h3,
	p {
		margin: 0;
	}
	h3 {
		font-size: 1rem;
	}
	section {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding-top: var(--space-1);
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.hint {
		color: var(--text-muted);
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.row select {
		flex: 1;
		min-width: 0;
	}
	.problem {
		color: var(--state-error);
		font-family: var(--font-mono);
		font-size: 0.75rem;
	}
</style>
