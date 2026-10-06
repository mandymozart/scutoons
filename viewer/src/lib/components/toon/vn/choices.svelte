<script lang="ts">
	import { goto } from '$app/navigation';
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte';
	import { getGameContext } from '$lib/contexts/game.svelte';
	import { runChoiceActions } from '$lib/visual-novel/variable-actions.ts';

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

	.vn-choice {
		background: white;
		color: black;
		padding: 1rem 1.5rem; /* control-ok: a dialogue choice may wrap */
		border: 2px solid lightgrey;
		cursor: pointer;
		font-size: 1rem;
		text-align: left;
	}

	.vn-choice:hover {
		background: white;
		border-color: #333;
	}

	.vn-choice:active {
		border-color: black;
	}
</style>
