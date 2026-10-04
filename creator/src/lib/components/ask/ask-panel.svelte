<script lang="ts">
	// The "Ask" panel (CUTOVER 10.28 step 2): a chat with an agent that has the signed-in creator's rights (the SCUTOONS
	// MCP tools). Docked on the right like the editor's side panels (drags wider up to half the window), full screen on
	// phones. Every draft the agent saved is a link to review it. DESIGN DRAFT: sample conversation, no backend yet.
	import { Icon, CloseButton } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { panelWidth } from '$lib/editor/panel-width.svelte.ts';

	let { context, onclose }: { context: string; onclose: () => void } = $props();

	const size = panelWidth('sc-panel-width:ask', 24);

	// the page makes room for the panel (owner 2026-10-04: "the chat pushes the page"): from 768px the shell's frame
	// ends at the panel's edge, so the editor stays usable next to the chat
	$effect(() => {
		const root = document.documentElement;
		root.classList.add('sc-ask-open');
		root.style.setProperty('--ask-width', size.css);
		return () => {
			root.classList.remove('sc-ask-open');
			root.style.removeProperty('--ask-width');
		};
	});

	type Step = { text: string; draft?: { label: string; href: string } };
	type Turn = { who: 'you' | 'agent'; text?: string; steps?: Step[] };

	// DRAFT: a sample of what the panel shows (the stroll request, CUTOVER 10.27)
	const turns: Turn[] = [
		{ who: 'you', text: 'Set up the chapter 2 stroll: every stroll scene ends with "end the stroll" or "keep walking" to a random other stroll scene, 15 minutes per scene, at 7:00 it ends.' },
		{
			who: 'agent',
			steps: [
				{ text: 'Read the rules: VN events, flow' },
				{ text: 'Chapter 2: 5 stroll scenes (scene-14 … scene-18), end scene-20' },
				{ text: 'scene-14 · de', draft: { label: 'Draft #812', href: '/catpope/chapter/2' } },
				{ text: 'scene-14 · en', draft: { label: 'Draft #813', href: '/catpope/chapter/2' } },
				{ text: 'scene-15 … scene-18 · de, en', draft: { label: '8 drafts', href: '/catpope/chapter/2' } },
				{ text: 'Checked: no errors, 0 warnings' }
			],
			text: 'Done. Each stroll scene\'s last slice now starts "stroll-end": +15 minutes, at 7:00 it jumps to scene-20, otherwise the reader picks "End the stroll" or "Keep walking". Nothing is published – review the drafts and publish when it reads right.'
		}
	];

	let input = $state('');
</script>

<!-- a focusable separator is a widget (WAI-ARIA window splitter) -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div class="resize" role="separator" aria-orientation="vertical" aria-label={m['creator.resize_panel']()} title={m['creator.resize_panel']()} tabindex="0" style:--panel-width={size.css} onpointerdown={size.start} onkeydown={size.key} ondblclick={size.reset}></div>
<aside class="ask" style:--panel-width={size.css} aria-label={m['creator.ask']()} data-testid="ask-panel">
	<header>
		<h2>{m['creator.ask']()}</h2>
		<button type="button" class="sc-round" title={m['creator.ask_new']()} aria-label={m['creator.ask_new']()}><Icon name="plus" /></button>
		<CloseButton label={m['creator.ask_close']()} onclick={onclose} />
	</header>

	<ol class="chat">
		{#each turns as turn, i (i)}
			<li class="turn {turn.who}">
				<p class="who">{turn.who === 'you' ? m['creator.ask_you']() : m['creator.ask_agent']()}</p>
				{#if turn.steps}
					<ul class="steps">
						{#each turn.steps as step, j (j)}
							<li>
								<Icon name={step.draft ? 'edit-3' : 'check'} size="1em" />
								<span>{step.text}</span>
								{#if step.draft}<a href={step.draft.href}>{step.draft.label}</a>{/if}
							</li>
						{/each}
					</ul>
				{/if}
				{#if turn.text}<p class="text">{turn.text}</p>{/if}
			</li>
		{/each}
	</ol>

	<form class="composer" onsubmit={(e) => e.preventDefault()}>
		<p class="context">{m['creator.ask_context']({ page: context })} · {m['creator.ask_drafts_only']()}</p>
		<div class="row">
			<textarea bind:value={input} rows="2" placeholder={m['creator.ask_placeholder']()} aria-label={m['creator.ask_placeholder']()}></textarea>
			<button type="submit" class="sc-round send" title={m['creator.ask_send']()} aria-label={m['creator.ask_send']()} disabled={!input.trim()}><Icon name="arrow-up" /></button>
		</div>
	</form>
</aside>

<style>
	.ask {
		position: fixed;
		top: var(--top);
		right: 0;
		bottom: var(--action-bar-height, 0px);
		width: var(--panel-width);
		z-index: 30;
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		background: var(--paper);
		border-left: var(--border-width-regular) solid var(--ink);
	}
	.resize {
		position: fixed;
		z-index: 31;
		top: var(--top);
		bottom: var(--action-bar-height, 0px);
		right: calc(var(--panel-width) - 0.375rem);
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: 0 var(--space-1);
		min-height: var(--control-height);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: 0;
		flex: 1;
	}
	.chat {
		flex: 1;
		overflow: auto;
		list-style: none;
		margin: 0;
		padding: var(--space-1);
		display: flex;
		flex-direction: column;
		gap: var(--space-15);
	}
	.who {
		margin: 0 0 var(--space-05);
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.turn.you .text {
		border-left: var(--border-width-regular) solid var(--ink);
		padding-left: var(--space-1);
	}
	.text {
		margin: 0;
		white-space: pre-wrap;
	}
	.steps {
		list-style: none;
		margin: 0 0 var(--space-1);
		padding: 0;
		font-size: var(--type-caption-size);
	}
	.steps li {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: 0.15rem 0;
		border-bottom: 1px solid var(--border-subtle);
	}
	.steps span {
		flex: 1;
	}
	.composer {
		border-top: var(--border-width-regular) solid var(--ink);
		padding: var(--space-05) var(--space-1) var(--space-1);
	}
	.context {
		margin: 0 0 var(--space-05);
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.row {
		display: flex;
		align-items: flex-end;
		gap: var(--space-05);
	}
	textarea {
		flex: 1;
		resize: none;
		min-height: var(--control-height);
		box-sizing: border-box;
	}
	@media (min-width: 768px) {
		:global(html.sc-ask-open .sc-shell-frame) {
			margin-right: var(--ask-width);
		}
	}
	@media (max-width: 767px) {
		.ask {
			left: 0;
			width: auto;
			border-left: 0;
		}
		.resize {
			display: none;
		}
	}
</style>
