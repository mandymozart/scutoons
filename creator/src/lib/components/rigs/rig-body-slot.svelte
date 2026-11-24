<script lang="ts">
	// A rig's 3D body: the model slot (a .glb from the asset manager) and its live preview – what the stage draws where the
	// rig stands. Saved with the rig's draft like any edit of the Assignment form.
	import { FileSlot, ModelPreview } from '@scutoons/media';
	import { setBody } from '@scutoons/rig-designer';
	import type { RigSession } from '$lib/editor/rig-session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { session }: { session: RigSession } = $props();

	const model = $derived(session.rig?.body?.model ?? null);
	const place = $derived(`rigs/${session.subject.replace(/^[a-z]+\./, '')}/`);

	function change(path: string | null) {
		if (session.rig) session.change(setBody($state.snapshot(session.rig), path));
	}
</script>

{#if session.rig}
	<div class="body" data-testid="rig-body">
		{#if model}<ModelPreview path={model} testid="rig-body-preview" />{/if}
		<div class="fields" title={m['creator.sound_body_hint']()}>
			<FileSlot kind="model" value={model} label={m['creator.sound_body']()} {place} player={false} onchange={change} testid="rig-body-model" />
		</div>
	</div>
{/if}

<style>
	.body {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		align-items: flex-start;
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--tool-text);
	}
	.fields {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		min-width: 0;
		max-width: 24rem;
	}
</style>
