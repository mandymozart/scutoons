<script lang="ts">
	// The shot's asset slot (lap 2 C3c; AI-HOOKS.md §0): a picture or a video – picked in the Library through the one
	// picker (files and generators that fit; Upload there), a file dropped on the slot (an upload, or a Library tile), or a
	// generator added from the Library (a picked one, or a new one named after the shot), which stands in until a take is
	// assigned – "Open" goes to the Generate workspace. The pick is stored in the shot as a Library address; empty, the slot shows the shot's approved generation.
	// What fits is the core's (library.takes).
	import { goto } from '$app/navigation';
	import { Icon } from '@scutoons/ui';
	import { Button, IconButton } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { library } from '$lib/editor/library.svelte.ts';
	import { dropAsset, isAssetDrag } from '$lib/editor/library-drop.ts';
	import { readShotAsset } from '@scutoons/content/storyboard';
	import { shotGenerations } from '$lib/editor/storyboard-generations.ts';
	import { pickedFile, slotContent } from '$lib/editor/storyboard-slot.ts';
	import { AREAS, hrefOf } from '$lib/shell/address.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import GenerationSlot from '$lib/components/storyboard/generation-slot.svelte';

	let { sb }: { sb: StoryboardPage } = $props();

	const FIT = { kinds: ['image', 'video'] as const };
	const entry = $derived(sb.sel);
	const approved = $derived(entry ? shotGenerations(sb.generations, entry.id).find((g) => g.approved && g.file) : undefined);
	const content = $derived(entry ? slotContent(entry.shot.asset, approved) : ({ at: 'empty' } as const));
	const aspect = $derived(entry?.shot.aspect ?? '');
	let over = $state(false);
	let busy = $state(false);

	function pick() {
		if (!entry) return;
		const id = entry.id;
		library.startPick({
			label: m['storyboard.slot_pick_for']({ id }),
			kinds: FIT.kinds,
			first: 'toon',
			value: content.at === 'file' ? content.path : undefined,
			onpick: (path) => store(id, pickedFile(path, library.familyOf(path))),
			things: { types: ['generator'], onthing: (t) => store(id, t.address), create: { label: () => m['library.new_generator'](), run: () => store(id, shotGenerator(id)) } }
		});
	}

	/** a new generator for the shot: it stands in the slot under the shot's name until a take is assigned */
	const shotGenerator = (id: string) => `generator:${id.toLowerCase().replace(/[^a-z0-9._-]+/g, '-').replace(/^[^a-z0-9]+/, '')}`;

	/** the shot's asset: a Library address (null or one the core doesn't read: not taken, the slot stays) */
	function store(id: string, address: string | null) {
		if (!address || !readShotAsset(address)) return void toast(m['storyboard.slot_not_taken'](), { type: 'error' });
		sb.setShotOf(id, ['asset'], address);
	}

	async function upload(file: File) {
		if (!entry) return;
		const id = entry.id;
		busy = true;
		try {
			await library.load();
			const { path } = await library.upload(file);
			store(id, pickedFile(path, library.familyOf(path)));
		} catch (e) {
			toast(m['storyboard.slot_upload_failed']({ reason: e instanceof Error ? e.message : String(e) }), { type: 'error' });
		} finally {
			busy = false;
		}
	}

	function ondrop(e: DragEvent) {
		over = false;
		if (!entry) return;
		const id = entry.id;
		if (isAssetDrag(e)) {
			dropAsset(e, { kinds: [...FIT.kinds] }, (path) => store(id, pickedFile(path, library.familyOf(path))));
			return;
		}
		const file = e.dataTransfer?.files?.[0];
		if (!file) return;
		e.preventDefault();
		void upload(file);
	}
	function ondragover(e: DragEvent) {
		if (!isAssetDrag(e) && !e.dataTransfer?.types.includes('Files')) return;
		e.preventDefault();
		over = true;
	}

	const fileName = (path: string) => path.split('/').pop() ?? path;
	const openGenerator = () => void goto(hrefOf(AREAS.generate) ?? '/generate');
</script>

{#if entry}
	<div
		class="slot"
		class:empty={content.at === 'empty'}
		class:over
		role="group"
		aria-label={m['storyboard.slot_title']({ id: entry.id })}
		title={m['storyboard.slot_drop_hint']()}
		{ondragover}
		ondragleave={() => (over = false)}
		{ondrop}
		data-testid="shot-asset-slot"
	>
		<span class="frame">
			{#if content.at === 'file' && content.video}
				<!-- svelte-ignore a11y_media_has_caption -->
				<video src="{asset(content.path)}#t=0.001" muted preload="metadata"></video>
			{:else if content.at === 'file'}
				<img src={asset(content.path)} alt="" />
			{:else if content.at === 'generator'}
				<Icon name="zap" />
			{:else if content.at === 'approved' && approved}
				<GenerationSlot {sb} g={approved} />
			{:else}
				<span class="dash">–</span>
			{/if}
		</span>
		<span class="words">
			{#if content.at === 'file'}
				<span class="name" title={content.path}>{fileName(content.path)}</span>
				<span class="sub">{content.video ? m['storyboard.slot_video']() : m['storyboard.slot_picture']()} · {aspect}</span>
			{:else if content.at === 'generator'}
				<span class="name" title={content.address}>{m['storyboard.slot_generator']({ name: content.name })}</span>
				<span class="sub">{m['storyboard.slot_stands_in']()}</span>
			{:else if content.at === 'approved'}
				<span class="name">{approved?.file ?? ''}</span>
				<span class="sub">{m['storyboard.slot_approved']()}</span>
			{:else}
				<span class="name">{busy ? m['storyboard.slot_uploading']() : m['storyboard.slot_empty']()}</span>
				<span class="sub">{m['storyboard.slot_takes']({ aspect })}</span>
			{/if}
		</span>
		{#if content.at === 'generator'}
			<Button title={m['storyboard.slot_open_hint']()} onclick={openGenerator} data-testid="slot-open">{m['storyboard.slot_open']()}</Button>
		{/if}
		<Button variant={content.at === 'empty' ? 'primary' : 'framed'} icon="grid" title={m['storyboard.slot_pick_hint']()} onclick={pick} data-testid="slot-pick">{m['storyboard.slot_pick']()}</Button>
		{#if entry.shot.asset}
			<IconButton label={m['storyboard.slot_clear']()} icon="x" tone="muted" onclick={() => sb.setShotOf(entry.id, ['asset'], null)} data-testid="slot-clear" />
		{/if}
	</div>
{/if}

<style>
	.slot {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
		padding: 0.45rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 0.4rem;
	}
	.slot.empty {
		border-style: dashed;
	}
	.slot.over {
		border-color: var(--kit-drop-on-border);
		background: var(--kit-drop-on-ground);
	}
	.frame {
		flex: none;
		display: grid;
		place-items: center;
		width: 2.75rem;
		height: 3.6rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 0.2rem;
		overflow: hidden;
		color: var(--tool-muted);
	}
	.frame img,
	.frame video {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.frame :global(.slot) {
		min-height: 0;
		width: 100%;
		height: 100%;
		border: 0;
	}
	.words {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sub {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
</style>
