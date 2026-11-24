<script lang="ts">
	import { untrack } from 'svelte';
	import { at, Button, Floats, FloatingPanels, Slider, Thumb, type PanelSpec, type TabSpec } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { setCreatorNav } from '$lib/nav.svelte.ts';
	import type { WorkspaceCard } from '$lib/shell/dashboard.ts';
	import type { DeckCard } from '$lib/shell/deck.ts';
	import { snapshots } from '$lib/shell/snapshots.svelte.ts';
	import { watched, type DocSession } from '$lib/editor/doc-session.svelte.ts';
	import { publishFlow, serverSource } from '$lib/editor/publish-flow.svelte.ts';
	import type { FilmGroup } from '$lib/editor/filmstrip.ts';
	import Card from '$lib/components/shell/workspace-card.svelte';
	import FullView from '$lib/components/shell/full-view.svelte';
	import PublishDialog from '$lib/components/shell/publish-dialog.svelte';
	import PublishLine from '$lib/components/shell/publish-line.svelte';
	import ReloadNotice from '$lib/components/shell/reload-notice.svelte';
	import OpenInDeck from '$lib/components/shell/open-in-deck.svelte';
	import StatusBar from '$lib/components/shell/status-bar.svelte';
	import Filmstrip from '$lib/components/filmstrip/filmstrip.svelte';
	import WritingPage from '$lib/components/writing/writing-page.svelte';
	import ReadParagraph from '$lib/components/writing/read-paragraph.svelte';
	import { publishLines, publishSamples, sampleStill } from '$lib/components/page/kit-samples.ts';

	/**
	 * The kit's parts that are more than a control (kit-d.html's rows after the controls, and what the live check found
	 * missing on /kit), each the creator's own component in its states; then the colours that stay, the colour-blind mode,
	 * and the rows that are the deck's. Samples only: nothing is sent or kept (kit-samples.ts).
	 */
	let { day, colorblind }: { day: boolean; colorblind: boolean } = $props();

	setCreatorNav();
	const STATES = ['rest', 'hover', 'on', 'focus', 'disabled', 'dragged', 'more'] as const;
	const stateName: Record<(typeof STATES)[number], () => string> = {
		rest: m['kit.state_rest'],
		hover: m['kit.state_hover'],
		on: m['kit.state_on'],
		focus: m['kit.state_focus'],
		disabled: m['kit.state_disabled'],
		dragged: m['kit.state_dragged'],
		more: m['kit.state_more']
	};
	const noop = () => {};

	// the workspace card's capture and the thumb's still: a drawn stage, held for this visit only
	const CAPTURED = 'kit-sample-captured';
	const still = $derived(sampleStill(day));
	$effect(() => {
		const url = still;
		untrack(() => (snapshots.local = { ...snapshots.local, [CAPTURED]: url }));
		return () => untrack(() => {
			const { [CAPTURED]: _gone, ...rest } = snapshots.local;
			snapshots.local = rest;
		});
	});
	const tabOf = (address: string, label: string) => ({ address, href: '#kit', label });
	const card = (id: string, front: boolean, open = true): WorkspaceCard => ({
		kind: 'character',
		id,
		href: open ? '#kit' : null,
		shown: open ? tabOf('character:crow-fox', 'crow-fox') : null,
		open: open ? [tabOf('character:crow-fox', 'crow-fox'), tabOf('character:mellowmantis', 'mellowmantis')] : [],
		front
	});
	const deckCard: DeckCard = { id: 'deck-stage', workspace: 'stage', item: 'scene:duet', href: '#kit' };

	// a panel head to try (hover, drag a tab, fold), and the same panel taken full
	const HEAD_TABS: TabSpec[] = [
		{ id: 'rig', name: 'Rig', icon: 'git-merge', about: '' },
		{ id: 'voice', name: 'Voice', icon: 'mic', about: '' },
		{ id: 'cast', name: 'Cast', icon: 'users', about: '' }
	];
	const HEAD_PANELS: PanelSpec[] = [{ id: 'rig', name: 'Rig', icon: 'git-merge', about: '', place: at('top-left', 0, 0), size: { w: 20, h: 'fill' }, min: { w: 12, h: 4 }, tabs: ['rig', 'voice', 'cast'] }];
	const floats = new Floats('kit-parts-head', HEAD_PANELS, HEAD_TABS);
	const tabName = (id: string) => HEAD_TABS.find((t) => t.id === id)?.name ?? id;

	// the Publish dialog on its sample lines; the server's source comes back when the page goes
	$effect(() => () =>
		untrack(() => {
			publishFlow.open = false;
			publishFlow.source = serverSource;
		})
	);
	function openPublish() {
		publishFlow.source = publishSamples;
		void publishFlow.show([], null);
	}
	const lines = publishLines();

	// the Reload notice of a sample session: a newer crow-fox while this page has unsaved changes
	$effect(() => {
		const was = untrack(() => watched.session);
		const sample = { notice: { doc: { entity: 'character:crow-fox' }, locale: '_', unsaved: true }, reload: noop, keepMine: noop };
		untrack(() => (watched.session = sample as unknown as DocSession));
		return () => untrack(() => (watched.session = was));
	});

	const film: FilmGroup[] = [
		{
			key: 'foyer',
			number: '3',
			label: 'Foyer',
			frames: [
				{ id: 'f1', label: 'Shot 1', detail: 'wide', meta: '4 s', aspect: 264 / 168 },
				{ id: 'f2', label: 'Shot 2', detail: 'close', meta: '2 s', aspect: 264 / 168 },
				{ id: 'f3', label: 'Shot 3', detail: 'over the shoulder', meta: '3 s', aspect: 9 / 16 },
				{ id: 'f4', label: 'Shot 4', meta: '1 s', aspect: 264 / 168 }
			]
		}
	];
	const frames = $derived(film.map((g) => ({ ...g, frames: g.frames.map((f) => (f.id === 'f4' ? f : { ...f, src: still })) })));
	const WRITING = '# The foyer\nThe lights go down one row at a time.\n\nCROW-FOX\n(whispering)\nThey left the *door* open again.\n\nMELLOWMANTIS\nThen we are **not** alone.';

	const SIGNALS = ['event', 'control', 'clock', 'audio', 'picture', 'address', 'vote', 'scene', 'far'];
	const OUTLETS = ['page', 'stage', 'target', 'device'];
	const GROUPS = [1, 2, 3, 4, 5, 6];
</script>

{#snippet head()}
	<span></span>
	{#each STATES as s (s)}<span class="head">{stateName[s]()}</span>{/each}
{/snippet}
{#snippet name(label: string, hint: string)}<span class="name" title={hint}>{label}</span>{/snippet}

<div class="parts" data-testid="kit-parts">
	{@render head()}

	{@render name(m['kit.workspace_card'](), m['kit.workspace_card_hint']())}
	<span class="card"><Card card={card(CAPTURED, false)} onopen={noop} onclose={noop} onfocus={noop} /></span>
	<span class="card"><Card card={card(CAPTURED, false)} onopen={noop} onclose={noop} onfocus={noop} data-kit-state="hover" /></span>
	<span class="card"><Card card={card(CAPTURED, true)} onopen={noop} onclose={noop} onfocus={noop} /></span>
	<span class="card"><Card card={card(CAPTURED, false)} onopen={noop} onclose={noop} onfocus={noop} data-kit-state="focus" /></span>
	<span></span>
	<span class="card"><Card card={card(CAPTURED, false)} onopen={noop} onclose={noop} onfocus={noop} data-kit-state="dragged" /></span>
	<span class="pair">
		<span class="card"><Card card={card('kit-sample-plain', false)} onopen={noop} onclose={noop} onfocus={noop} /></span>
		<span class="card"><Card card={card('kit-sample-empty', false, false)} onopen={noop} onclose={noop} onfocus={noop} /></span>
	</span>

	{@render name(m['kit.thumb'](), m['kit.thumb_hint']())}
	<span class="thumb"><Thumb src={still} /></span>
	<span class="thumb"><Thumb src={still} data-kit-state="hover" /></span>
	<span class="thumb"><Thumb src={still} on /></span>
	<span class="thumb"><Thumb src={still} data-kit-state="focus" /></span>
	<span class="thumb"><Thumb src={still} disabled /></span>
	<span class="thumb"><Thumb src={still} data-kit-state="dragged" /></span>
	<span class="thumb"><Thumb icon="user" empty={m['shell.no_still']()} /></span>

	{@render name(m['kit.panel_head'](), m['kit.panel_head_hint']())}
	<span class="wide row">
		<span class="frame head-frame">
			<FloatingPanels {floats}>
				{#snippet tab(id)}<p class="muted pad">{tabName(id)}</p>{/snippet}
			</FloatingPanels>
		</span>
		<span class="muted">{m['kit.live']()}</span>
	</span>

	{@render name(m['kit.overlay'](), m['kit.overlay_hint']())}
	<span class="wide row">
		<Button onclick={() => floats.setState('voice', 'full')} data-testid="kit-parts-full">{m['kit.overlay_full']()}</Button>
		<Button onclick={openPublish} data-testid="kit-parts-publish">{m['kit.overlay_dialog']()}</Button>
	</span>

	{@render name(m['kit.publish_row'](), m['kit.publish_row_hint']())}
	<span class="line"><PublishLine line={lines[0]} on ontick={noop} title={lines[0].hint} /></span>
	<span></span>
	<span></span>
	<span></span>
	<span class="line"><PublishLine line={lines[0]} on disabled ontick={noop} /></span>
	<span></span>
	<span class="line"><PublishLine line={lines[5]} on={false} ontick={noop} title={lines[5].hint} /></span>

	{@render name(m['kit.filmstrip'](), m['kit.filmstrip_hint']())}
	<span class="wide row top">
		<span class="film"><Filmstrip groups={frames} current="f2" picked={['f2', 'f3']} onpick={noop} label={m['kit.filmstrip']()} empty={m['shell.no_still']()} testid="kit-film" /></span>
		<span class="film-row"><Filmstrip groups={frames} current="f1" onpick={noop} row label={m['kit.filmstrip']()} empty={m['shell.no_still']()} testid="kit-film-row" /></span>
	</span>

	{@render name(m['kit.writing'](), m['kit.writing_hint']())}
	<span class="wide frame writing"><WritingPage title="Feilchen" sub="Chapter 3 · Script" testid="kit-writing"><ReadParagraph text={WRITING} lang="en" /></WritingPage></span>

	{@render name(m['kit.reload'](), m['kit.reload_hint']())}
	<span class="wide frame fixed short above"><ReloadNotice /></span>

	{@render name(m['kit.status_bar'](), m['kit.status_bar_hint']())}
	<span class="wide frame fixed short"><StatusBar /></span>

	{@render name(m['kit.deck_link'](), m['kit.deck_link_hint']())}
	<span><OpenInDeck address="scene:duet" testid="kit-open-in-deck" /></span>
	<span></span><span></span><span></span><span></span><span></span>
	<span class="card"><Card card={deckCard} onopen={noop} onclose={noop} onfocus={noop} /></span>

	{@render name(m['kit.slider'](), m['kit.slider_hint']())}
	<span><Slider value={40} label={m['kit.slider']()} /></span>
	<span class="muted">{m['kit.live']()}</span>
	<span></span>
	<span></span>
	<span><Slider value={40} disabled label={m['kit.slider']()} /></span>
	<span></span><span></span>

	{@render name(m['kit.signals'](), m['kit.signals_hint']())}
	<span class="wide row">{#each SIGNALS as s (s)}<span class="swatch"><i style:background="var(--signal-{s})"></i>{s}</span>{/each}</span>

	{@render name(m['kit.outlets'](), m['kit.outlets_hint']())}
	<span class="wide row">{#each OUTLETS as o (o)}<span class="swatch"><i style:background="var(--outlet-{o})"></i>{o}</span>{/each}</span>

	{@render name(m['kit.groups'](), m['kit.groups_hint']())}
	<span class="wide row">{#each GROUPS as g (g)}<span class="swatch"><i style:background="var(--group-{g})"></i>{g}</span>{/each}</span>

	<span class="name">{m['kit.colorblind']()}</span>
	<span class="wide note" data-testid="kit-colorblind">{colorblind ? m['kit.colorblind_on']() : m['kit.colorblind_off']()} {m['kit.colorblind_only']()}</span>

	{@render name(m['kit.decks'](), m['kit.decks_hint']())}
	<span class="wide note" title={m['kit.decks_hint']()}>{m['kit.decks_rows']()}</span>
</div>

{#if floats.full}<FullView {floats} full={floats.full} name={tabName} has={() => true} content={view} />{/if}
{#snippet view(id: string)}<p class="muted">{tabName(id)}</p>{/snippet}
{#if publishFlow.open}<PublishDialog />{/if}

<style>
	.parts {
		display: grid;
		grid-template-columns: 8rem repeat(7, minmax(9rem, max-content));
		align-items: center;
		gap: 0.75rem 1rem;
		padding: 0 2rem 3rem;
		color: var(--tool-text);
	}
	.head {
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.name,
	.muted,
	.note {
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.name {
		cursor: help;
	}
	.wide {
		grid-column: span 7;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 1rem;
	}
	.row.top {
		align-items: flex-start;
	}
	.pad {
		margin: 0;
		padding: var(--editor-pad);
	}
	.card {
		display: block;
		width: 11rem;
	}
	.pair {
		display: flex;
		gap: 0.75rem;
	}
	.thumb {
		display: block;
		width: 9rem;
	}
	.line {
		display: block;
		width: 17rem;
	}
	/* a part that places itself on the window (fixed) stays in its frame here: the frame is its containing block */
	.frame {
		position: relative;
		display: block;
		border: var(--editor-edge) dashed var(--tool-line);
		border-radius: var(--kit-radius-box);
	}
	.frame.fixed {
		transform: translateZ(0);
	}
	.frame.short {
		height: calc(var(--kit-bar) + 2 * var(--kit-margin));
		max-width: 52rem;
	}
	/* the notice keeps the status bar's room under it */
	.frame.above {
		height: calc(2 * var(--kit-bar) + 3 * var(--kit-margin));
	}
	.head-frame {
		width: 30rem;
		height: 9rem;
	}
	.writing {
		max-width: 52rem;
		max-height: 20rem;
		overflow: auto;
	}
	.film {
		display: block;
		width: 16rem;
	}
	.film-row {
		display: block;
		width: 30rem;
	}
	.swatch {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		font-size: var(--editor-small);
	}
	.swatch i {
		width: 0.75rem;
		height: 0.75rem;
		border-radius: 50%;
	}
	.note {
		max-width: 52rem;
	}
</style>
