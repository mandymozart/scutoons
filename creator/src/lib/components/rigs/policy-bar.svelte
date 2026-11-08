<script lang="ts">
	// The Rust tab on a rig (#412, research/RIG-MODULE-GROUPS.md §2 Q4): the rig's policy – its own `module:<id>` document,
	// edited with the module editor below. "New policy" starts one from the core's template when the rig has none; "Open
	// policy" comes back to it after another module was picked. The core's word on the draft stands here too: why it does
	// not read yet ("compile it first") or what the browser's deck leaves out of it (no module runtime before #290).
	import { askDialog } from '@scutoons/ui';
	import { putDraft } from '$lib/api.ts';
	import { firstDraft, idProblem } from '$lib/editor/module-catalog.ts';
	import { moduleKey } from '$lib/editor/module-store.ts';
	import { policyIdFor } from '$lib/editor/rig-assign.ts';
	import type { RigSession } from '$lib/editor/rig-session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { session, open, onselect }: {
		session: RigSession;
		/** the module the editor has open */
		open: string | null;
		/** opens a module in the editor */
		onselect: (id: string) => void;
	} = $props();

	const policy = $derived(session.policy);
	let notice = $state('');

	/** a module of its own for the rig, its first files the core's template; the rig names it, it opens */
	async function newPolicy(): Promise<void> {
		const typed = await askDialog({ title: m['creator.sound_new_policy'](), message: m['creator.sound_new_policy_hint']({ subject: session.subject }), label: 'id', value: policyIdFor(session.subject), select: true, confirmLabel: m['creator.sound_new_module_create']() });
		const id = typed?.trim() ?? '';
		if (!id) return;
		await session.loadLists();
		const problem = idProblem(id, session.moduleIds);
		notice = problem === 'invalid' ? m['creator.sound_id_invalid']({ id }) : problem === 'taken' ? m['creator.sound_id_taken']({ id }) : '';
		if (problem) return;
		try {
			await putDraft(moduleKey(id), firstDraft(id, null, await session.policyFiles(id)), null, `policy of rig:${session.subject}`);
		} catch (e) {
			return void (notice = e instanceof Error ? e.message : String(e));
		}
		const attached = await session.attachPolicy(id);
		if (!attached.ok) notice = attached.message;
		onselect(id);
	}
</script>

<div class="bar" data-testid="sound-policy">
	<span class="line">
		{#if !session.rig}{m['creator.sound_policy_needs_rig']()}{:else if policy}{m['creator.sound_policy_of']({ id: policy })}{:else}{m['creator.sound_policy_none']()}{/if}
	</span>
	{#if session.rig && !policy}
		<button type="button" onclick={newPolicy} data-testid="sound-new-policy">{m['creator.sound_new_policy']()}</button>
	{:else if policy && policy !== open}
		<button type="button" onclick={() => onselect(policy)} data-testid="sound-open-policy">{m['creator.sound_policy_open']()}</button>
	{/if}
</div>
{#if notice || session.problem}<p class="said" role="status" data-testid="sound-policy-status">{notice || session.problem}</p>{/if}

<style>
	.bar {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		font-family: var(--font-mono);
		font-size: 0.75rem;
	}
	.line {
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.said {
		flex: none;
		margin: 0;
		padding: var(--space-025) var(--space-1);
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
</style>
