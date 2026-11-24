<script lang="ts">
	// The Assignment tab of the Sound page (#283): the open character's rig `rig:<subject>` as the designer's form, saved as
	// a draft on its own a moment after each edit and published by the bar's one Publish with what it plays (rig-session.svelte.ts holds it, the Rust tab shares it: a code rig's
	// policy, #412), heard on the deck half at every change, and the characters without a default rig (the core's coverage
	// over the registry and the catalog's rig list). What a policy owns the form shows read-only, from the core's face.
	import { AssignmentTab, RigCoverageList, designerMessages as m } from '@scutoons/rig-designer';
	import { Button } from '@scutoons/ui/editor';
	import { editHref } from '$lib/editor/edit-key.ts';
	import { rigKey } from '$lib/editor/rig-assign.ts';
	import RigBodySlot from '$lib/components/rigs/rig-body-slot.svelte';
	import type { PlayHost, RigSession } from '$lib/editor/rig-session.svelte.ts';

	let { session, host, moods = [] }: {
		session: RigSession;
		/** the deck half's engine: the draft is heard there */
		host: PlayHost | null;
		/** the character's moods (the deck's) */
		moods?: string[];
	} = $props();

	$effect(() => {
		session.host = host;
	});

</script>

<div class="pane" data-testid="assignment-pane">
	<RigBodySlot {session} />
	<div class="form">
		<AssignmentTab rig={session.rig} instruments={session.lists.instruments} presets={session.lists.presets} {moods} coverage={session.mine} problem={session.problem} face={session.face} onchange={(next) => session.change(next)} oncreate={() => session.create()} />
	</div>
	<RigCoverageList list={session.coverage} current={session.subject} href={(c) => editHref(rigKey(c.subject), { w: 'sound' })} />
	{#if session.rig}
		<footer class="actions">
			<Button onclick={() => session.hear()} title={m['designer.assign_hear_title']()} data-testid="assign-hear">{m['designer.assign_hear']()}</Button>
		</footer>
	{/if}
</div>

<style>
	.pane {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	.form {
		flex: 1;
		min-height: 0;
		display: grid;
		overflow: hidden;
	}
	.actions {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
		border-top: 1px solid var(--tool-line);
	}
</style>
