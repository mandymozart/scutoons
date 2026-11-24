<script lang="ts">
	// Sound design's views (lap 2 T10, RULES H5): the code (`rust`, the page's), Parameters, Nodes, the rig's Assignment,
	// the Monitor, Out, Knobs, Problems, Console – each a tab of the workspace host (lib/shell/presets.ts `sound`). The deck
	// playing the rig lives here while the page does: the Monitor view borrows its element wherever it is shown (a panel,
	// the stage, taken full); folded or closed, the deck plays on and Out, Knobs and Nodes keep reading it.
	import { onDestroy, type Snippet } from 'svelte';
	import { ConsolePane, NodesTab, ParametersTab, ProblemsPane, type RigDesigner } from '@scutoons/rig-designer';
	import { useTabs } from '$lib/shell/shell.svelte.ts';
	import { SoundReadout } from '$lib/editor/sound-readout.svelte.ts';
	import type { RigSession } from '$lib/editor/rig-session.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import AssignmentPane from '$lib/components/rigs/assignment-pane.svelte';
	import KnobsStrip from '$lib/components/sound/knobs-strip.svelte';
	import SignalStrip from '$lib/components/sound/signal-strip.svelte';
	import SoundMonitor from '$lib/components/sound/sound-monitor.svelte';

	let { designer, session = null, character, rust }: { designer: RigDesigner; session?: RigSession | null; character: string; rust: Snippet } = $props();

	const readout = new SoundReadout(getLocale);
	let keep = $state<HTMLElement>();
	let monitor = $state<HTMLElement>();
	/** the Monitor's slot takes the deck's element; it goes back to the keep when the slot goes */
	function lend(slot: HTMLElement) {
		const el = monitor;
		if (!el) return;
		slot.appendChild(el);
		return () => {
			if (el.parentElement === slot && keep?.isConnected) keep.appendChild(el);
		};
	}
	onDestroy(() => monitor?.remove());

	useTabs(() => ({
		rust,
		parameters: parametersTab,
		nodes: nodesTab,
		assignment: session ? assignmentTab : null,
		monitor: monitorTab,
		meters: metersTab,
		knobs: knobsTab,
		diagnostics: problemsTab,
		console: consoleTab
	}));
</script>

{#snippet parametersTab()}
	{#if designer.moduleId}<ParametersTab {designer} />{:else}<p class="sc-empty pad">{m['creator.sound_pick_module']()}</p>{/if}
{/snippet}

{#snippet nodesTab()}
	<NodesTab wiring={readout.wiring} read={(p) => readout.host?.deck?.read(p) ?? NaN} active />
{/snippet}

{#snippet assignmentTab()}
	{#if session}<AssignmentPane {session} host={readout.host} moods={readout.host?.moods ?? []} />{/if}
{/snippet}

{#snippet monitorTab()}
	<div class="lent" {@attach lend}></div>
{/snippet}

{#snippet metersTab()}
	<SignalStrip inputs={readout.inputs} outputs={readout.outputs} reduction={readout.reduction} signals={readout.signals} />
{/snippet}

{#snippet knobsTab()}
	<KnobsStrip knobs={readout.columns} />
{/snippet}

{#snippet problemsTab()}
	<ProblemsPane {designer} />
{/snippet}

{#snippet consoleTab()}
	<ConsolePane {designer} />
{/snippet}

<div class="keep" bind:this={keep} data-testid="sound-workspace">
	<div class="monitor" bind:this={monitor}><SoundMonitor {character} {readout} /></div>
</div>

<style>
	/* where the deck waits while no view shows it: off the window, at a size its picture can be drawn in */
	.keep {
		position: fixed;
		left: -200vw;
		top: 0;
		width: 26rem;
		height: 18rem;
		visibility: hidden;
		pointer-events: none;
	}
	.monitor {
		display: flex;
		width: 100%;
		height: 100%;
		min-height: 0;
		visibility: visible;
		pointer-events: auto;
	}
	.keep .monitor {
		visibility: hidden;
	}
	.lent {
		display: flex;
		flex: 1;
		min-height: 0;
	}
	.pad {
		padding: var(--space-1);
	}
</style>
