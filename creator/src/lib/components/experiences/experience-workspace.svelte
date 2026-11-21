<script lang="ts">
	import { untrack } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { NEUTRAL_LOCALE } from '@scutoons/content';
	import { publishMany } from '$lib/api.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { experienceFacts, experienceHref, experienceKey, experienceOutline, titleIn } from '$lib/editor/experiences.ts';
	import { neutralDoc } from '$lib/editor/languages.ts';
	import { publishTogether } from '$lib/editor/publish-together.ts';
	import { publishOutcome } from '$lib/editor/scenes.ts';
	import { STAGE_LOADING, type StageView } from '$lib/editor/xr-stage-frame.ts';
	import { getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import { shell, usePageDoc, useTabs } from '$lib/shell/shell.svelte.ts';
	import { toast } from '$lib/toasts.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import ExperienceMeta from './experience-meta.svelte';
	import ExperienceOutline from './experience-outline.svelte';
	import ExperienceScenes from './experience-scenes.svelte';
	import ExperienceStage from './experience-stage.svelte';

	/**
	 * The XR editor's workspace (#482): the stage is the experience and fills the whole workspace; the panels lie over it –
	 * the outline in the Tree, the meta in the Experience tab, the scenes strip with the stage's view in the Scenes tab;
	 * Publish and History are the bar's, as everywhere. Experiences are opened from the Library. View only so far.
	 */
	let { id }: { id: string | null } = $props();

	const nav = getCreatorNav();
	const locale = getLocale();
	let doc = $state.raw<ContentDoc | null>(null);
	let picked = $state('');
	let view = $state<StageView>('reader');
	let run = $state(0);
	let shown = $state(STAGE_LOADING);
	let problems = $state<string[]>([]);
	let busy = $state(false);

	$effect(() => {
		const current = id;
		untrack(() => {
			problems = [];
			picked = '';
			doc = current ? neutralDoc(experienceKey(current)) : null;
			void doc?.load();
		});
	});

	const content = $derived(doc && !doc.loading ? (doc.data[NEUTRAL_LOCALE] ?? null) : null);
	const facts = $derived(content && id ? experienceFacts(content, id) : null);
	const scenes = $derived(experienceOutline((content as { configuration?: unknown } | null)?.configuration).scenes);
	const scene = $derived(scenes.find((s) => s.id === picked)?.id ?? scenes[0]?.id ?? '');
	const base = $derived(doc?.base[NEUTRAL_LOCALE] ?? null);
	const unpublished = $derived(!!doc && base !== null && doc.published[NEUTRAL_LOCALE] !== base);

	const TYPES: Record<string, () => string> = {
		book: () => m['creator.experiences_type_book'](),
		record: () => m['creator.experiences_type_record'](),
		poster: () => m['creator.experiences_type_poster'](),
		exhibition: () => m['creator.experiences_type_exhibition'](),
		'concert-reading': () => m['creator.experiences_type_concert_reading']()
	};

	/** the creator's one way to publish (publish-together.ts): the server's publish-many, refusals named */
	async function publish() {
		if (!doc || base === null || busy) return;
		const open = doc;
		busy = true;
		try {
			const together = await publishTogether([{ key: open.key(NEUTRAL_LOCALE), version: base }], publishMany);
			problems = together.refused.map((r) => r.why);
			if (together.done.length) toast(m['creator.experiences_publish_ok'](), { type: 'success' });
		} catch (e) {
			const outcome = publishOutcome(e);
			problems = outcome.kind === 'problems' ? outcome.problems : [outcome.kind === 'error' ? outcome.message : m['creator.experiences_unreachable']()];
		} finally {
			busy = false;
		}
		await open.load();
	}

	usePageNav({ fill: true, extra: () => (facts ? [{ href: experienceHref(facts.id), label: titleIn(facts.title, locale, facts.id) }] : []) });
	usePageDoc(() => (doc ? { publish: publishMenu, docs: [doc], unpublished, errors: problems.length } : null));
	useTabs(() => ({ tree: facts ? tree : null, experience: facts ? meta : null, scenes: facts && scenes.length ? strip : null }));
</script>

{#snippet publishMenu()}
	<div class="publish-menu">
		<button type="button" class:primary={unpublished} disabled={!unpublished || busy} onclick={() => void publish()} data-testid="experience-publish"><Icon name="upload-cloud" />{m['creator.publish']()}</button>
		{#if !unpublished}<p class="muted">{m['creator.experiences_nothing_new']()}</p>{/if}
	</div>
{/snippet}

{#snippet tree()}
	{#if facts}<ExperienceOutline {content} assets={facts.assets} />{/if}
{/snippet}

{#snippet meta()}
	{#if facts}<ExperienceMeta {facts} typeName={TYPES[facts.type]?.() ?? facts.type} toons={facts.toons.map((t) => nav.toonName(t)).join(' · ')} {problems} />{/if}
{/snippet}

{#snippet strip()}
	{#if facts}<ExperienceScenes {scenes} current={scene} assets={facts.assets} bind:view playing={shown.phase === 'playing'} onpick={(s) => ((picked = s), (view = 'reader'))} onreplay={() => ((run += 1), (view = 'reader'))} />{/if}
{/snippet}

<div class="xr-workspace" data-testid="experience-workspace">
	{#if facts && scene}
		<ExperienceStage {content} assets={facts.assets} {scene} {view} {run} bind:shown />
	{:else}
		<div class="empty">
			{#if !id}
				<p>{m['creator.experiences_from_library']()}</p>
				<button type="button" onclick={() => shell.open('library')} data-testid="experience-open-library"><Icon name="grid" />{m['shell.library']()}</button>
			{:else if doc?.error}
				<p role="alert">{doc.error}</p>
			{:else if doc && !doc.loading}
				<p>{m['creator.experiences_stage_none']()}</p>
			{:else}
				<p role="status">{m['creator.experiences_stage_loading']()}</p>
			{/if}
		</div>
	{/if}
</div>

<style>
	.xr-workspace {
		position: fixed;
		inset: 0;
		z-index: 0;
		background: var(--editor-stage, var(--ink));
	}
	.empty {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		color: var(--text-muted);
	}
	.empty p {
		margin: 0;
	}
	.publish-menu {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.publish-menu p {
		margin: 0;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
