<script lang="ts">
	import { setContext, untrack } from 'svelte';
	import { Button, Floats, FloatingPanels, Pill, Toggle } from '@scutoons/ui/editor';
	import { contextKey } from '$lib/utils/shared.ts';
	import { availableTabs, panelSpecs, presetStart, tabSpecs } from '$lib/shell/panels.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { setCreatorNav } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { publishFlow, serverSource, type PublishSource } from '$lib/editor/publish-flow.svelte.ts';
	import type { PublishLine } from '$lib/editor/publish-tree.ts';
	import ShellBar from '$lib/components/shell/shell-bar.svelte';
	import Dashboard from '$lib/components/shell/dashboard.svelte';
	import Launcher from '$lib/components/shell/launcher.svelte';
	import FullView from '$lib/components/shell/full-view.svelte';

	/**
	 * Dev only (the /kit page's Shell): the creator's bar, its ☰ menu with Views and the user menu, without a sign-in – a
	 * test account, a character open in its workspace with the character views filled, something to undo and to publish
	 * (the Publish dialog on sample lines – nothing is sent), three text languages. The Translator switch gives the bar the translator's craft; the overlay level's three – the
	 * switcher, Search, a view taken full – open from the foot (Ctrl K and Alt Space are the creator's own).
	 */
	let { onleave }: { onleave: () => void } = $props();

	setContext(contextKey('creator-session'), { user: { email: 'agent@scutoons.test' }, roles: ['admin', 'creator'], logout: () => {} });
	setCreatorNav();

	const FILLED = ['sheet', '3d', 'kanon', 'expressions', 'rig', 'voice', 'cast', 'generations'];
	const specs = tabSpecs();
	const available = availableTabs(FILLED, null);
	const floats = new Floats('kit-shell-character', panelSpecs('character', available, specs), specs, presetStart('character'));
	let translator = $state(false);

	// the Publish dialog's samples (lap 2 A10): the bar's icon lists them, its primary answers with a sample report
	const line = (id: string, label: string, chip: string, type: string): PublishLine => ({ id, label, chip, type, hint: 'v12 · agent@scutoons.test', review: false });
	const changed = m['creator.publish_part_changed']();
	const added = m['creator.publish_part_added']();
	const SAMPLES: PublishLine[] = [
		line('character:sandgirl', 'Sandgirl', changed, 'Character'),
		line('character:giantpopstar', 'Giant Popstar', changed, 'Character'),
		line('character:mellowmantis', 'Mellowmantis', changed, 'Character'),
		line('chapter:catpope:3:de', 'Chapter 3 (de)', changed, 'Chapter · Catpope'),
		line('season:catpope:1', 'Season 1', changed, 'Season · Catpope'),
		line('place:foyer', 'Foyer', added, 'Place'),
		line('module:walk', 'walk', added, 'Module'),
		line('show:top-toy', 'top-toy', changed, 'Show')
	];
	const samples: PublishSource = {
		load: async () => ({ files: [], lines: SAMPLES }),
		send: async (list, chosen) => {
			const out = list.lines.filter((l) => chosen.has(l.id) && l.id !== 'show:top-toy').map((l) => l.label);
			const notices = [
				{ type: 'success' as const, text: m['creator.publish_tree_done_named']({ count: out.length, names: out.join(', ') }) },
				...(chosen.has('show:top-toy') ? [{ type: 'error' as const, text: m['creator.publish_tree_refused']({ name: 'top-toy', reason: 'needs module:walk-2, which is not published' }) }] : [])
			];
			return { message: notices[0].text, published: [], refused: [], notices };
		}
	};

	// the sample keeps nothing: the real tabs, workspaces, layouts and settings come back as they were when it ends
	$effect(() => {
		const end = untrack(() => shell.sample());
		untrack(() => {
			shell.visit('character:crow-fox', '/characters?id=crow-fox', 'crow-fox');
			shell.visit('character:mellowmantis', '/characters?id=mellowmantis', 'mellowmantis');
			shell.visit('character:giantpopstar', '/characters?id=giantpopstar', 'giantpopstar');
			shell.floats = floats;
			shell.kind = 'character';
			shell.available = available;
			textLanguage.page = { languages: ['de', 'en', 'ko'], source: 'de' };
			shell.doc = { unpublished: true, undo: () => {}, redo: () => {} };
			publishFlow.source = samples;
		});
		return () => {
			shell.floats = null;
			shell.doc = null;
			publishFlow.source = serverSource;
			publishFlow.open = false;
			shell.menu = false;
			shell.userMenu = false;
			shell.toggleSwitcher(false);
			shell.toggleLauncher(false);
			floats.full = null;
			crafts.mine = null;
			textLanguage.page = { languages: [], source: 'de' };
			end();
		};
	});
	$effect(() => {
		crafts.mine = translator ? { crafts: ['translator'], invited: [], workspaces: [], ask: false, can_edit: false } : null;
	});
	const name = (id: string) => specs.find((t) => t.id === id)?.name ?? id;
</script>

{#snippet view(id: string)}<p class="view">{name(id)}</p>{/snippet}

<div class="kit-shell">
	<ShellBar />
	<div class="room">
		<FloatingPanels {floats}>
			{#snippet tab(id)}{@render view(id)}{/snippet}
		</FloatingPanels>
	</div>
	{#if floats.full}<FullView {floats} full={floats.full} {name} has={(t) => FILLED.includes(t)} content={view} />{/if}
	{#if shell.dashboard}<Dashboard />{/if}
	{#if shell.launcher}<Launcher />{/if}
	<div class="foot">
		<Pill label="Kit">
			<span class="word">Translator</span>
			<Toggle on={translator} label="Translator" onchange={(on) => (translator = on)} />
			<Button variant="quiet" onclick={() => shell.toggleSwitcher(true)} data-testid="kit-switcher">Switcher</Button>
			<Button variant="quiet" onclick={() => shell.toggleLauncher(true)} data-testid="kit-search">Search</Button>
			<Button variant="quiet" onclick={() => floats.setState('rig', 'full')} data-testid="kit-full">Full</Button>
			<Button variant="quiet" onclick={onleave}>Kit</Button>
		</Pill>
	</div>
</div>

<style>
	.kit-shell {
		--shell-top: calc(var(--kit-bar) + 2 * var(--kit-margin));
		--status-room: calc(var(--kit-status) + 2 * var(--kit-margin));
		position: fixed;
		inset: 0;
		background: var(--tool-ground);
	}
	.room {
		position: absolute;
		inset: var(--shell-top) 0 var(--status-room);
	}
	.view {
		margin: 0;
		padding: var(--editor-pad);
		color: var(--tool-muted);
	}
	.foot {
		position: absolute;
		left: var(--editor-gap);
		bottom: var(--kit-margin);
	}
	.word {
		padding: 0 calc(var(--editor-pad) / 2);
	}
</style>
