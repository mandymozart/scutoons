<script lang="ts">
	// The Assignment tab of the Sound page (#283): the open character's rig `rig:<subject>` as the designer's form, saved as
	// a draft and published like any catalog document (rig-session.svelte.ts holds it, the Rust tab shares it: a code rig's
	// policy, #412), heard on the deck half at every change, and the characters without a default rig (the core's coverage
	// over the registry and the catalog's rig list). What a policy owns the form shows read-only, from the core's face.
	import { AssignmentTab, RigCoverageList, designerMessages as m } from '@scutoons/rig-designer';
	import { confirmDialog } from '@scutoons/ui';
	import { editHref } from '$lib/editor/edit-key.ts';
	import { rigKey } from '$lib/editor/rig-assign.ts';
	import type { PlayHost, RigSession } from '$lib/editor/rig-session.svelte.ts';
	import { toast } from '$lib/toasts.ts';

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

	async function save(): Promise<void> {
		const r = await session.save();
		if (!r.ok) toast(r.message, { type: 'error' });
	}

	async function publish(): Promise<void> {
		if (!session.rig || session.dirty || session.busy) return;
		if (!(await confirmDialog({ title: m['designer.assign_publish'](), message: m['designer.assign_publish_confirm']({ subject: session.subject }), confirmLabel: m['designer.assign_publish']() }))) return;
		const r = await session.publish();
		if (r.ok) toast(m['designer.assign_published']({ subject: session.subject }));
		else toast(r.message, { type: 'error' });
	}
</script>

<div class="pane" data-testid="assignment-pane">
	<div class="form">
		<AssignmentTab rig={session.rig} instruments={session.lists.instruments} presets={session.lists.presets} {moods} coverage={session.mine} problem={session.problem} face={session.face} onchange={(next) => session.change(next)} oncreate={() => session.create()} />
	</div>
	<RigCoverageList list={session.coverage} current={session.subject} href={(c) => editHref(rigKey(c.subject), { w: 'sound' })} />
	{#if session.rig}
		<footer class="actions">
			<span class="state">{session.dirty ? m['designer.unsaved']() : ''}</span>
			<button type="button" onclick={() => session.hear()} title={m['designer.assign_hear_title']()} data-testid="assign-hear">{m['designer.assign_hear']()}</button>
			<button type="button" onclick={publish} disabled={session.dirty || session.busy || !session.draftWaiting} title={m['designer.assign_publish_title']()} data-testid="assign-publish">{m['designer.assign_publish']()}</button>
			<button type="button" onclick={save} disabled={!session.dirty || session.busy} title={m['designer.assign_save_title']()} data-testid="assign-save">{m['designer.save']()}</button>
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
	}
	.actions {
		flex: none;
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
		border-top: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	.state {
		flex: 1;
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.75rem;
	}
</style>
