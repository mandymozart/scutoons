<script lang="ts">
	import { onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { runChoiceActions } from '$lib/visual-novel/variable-actions.ts';
	import { listenPick } from '$lib/reader-story.ts';

	const vn = getVisualNovelContext();
	const game = getGameContext();

	const choices = $derived(vn.state.pendingChoices);
	const visible = $derived(vn.state.choicesVisible && choices.length > 0);

	function handleSelect(choice: typeof choices[0]) {
		// the choice's actions in order: variables, then a jump (navigates) or a divert (the script continues there)
		// a jump into a group that isn't there is no jump (the script goes on – no dead end)
		const { jumpTo, divert } = runChoiceActions(choice.onSelect, game, choice.from ?? vn.scriptLabel, (target) => vn.jumpProblem(target));
		if (divert) vn.setPendingDivert(divert);

		if (jumpTo) {
			console.log(`🎯 Choice selected, navigating to: ${jumpTo}`);
			vn.setNavigationTriggered();
			vn.clearChoices();
			goto(jumpTo);
		} else {
			vn.clearChoices();
		}
	}

	// #552: a linked reader's vote result picks the shown choice at the place the core named (reader-session.ts), as a tap
	const pick = (index: number) => {
		const choice = visible ? choices[index] : undefined;
		if (choice) handleSelect(choice);
		return !!choice;
	};
	onDestroy(listenPick(pick));
</script>

<div class="vn-choices" class:visible>
	{#each choices as choice}
		<button class="vn-choice" data-control-exempt="dialogue choice (may wrap)" onclick={() => handleSelect(choice)}>
			{choice.label}
		</button>
	{/each}
</div>

<style>
	.vn-choices {
		display: none;
		flex-direction: column;
		gap: 0.75rem;
	}

	.vn-choices.visible {
		display: flex;
	}

	/* the choice look: vn/message.svelte (shared with the chapter's end actions) */
</style>
