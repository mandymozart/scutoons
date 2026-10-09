<script>
	// The shot's frame, its variants (pick one = it becomes the shot, the old one a variant; load = into the editor) and
	// the generated images (Luma results, photos, drawings) dropped onto it – one approved = the chosen image (dataset).
	import { askDialog } from '@scutoons/ui';
	import { api, ch, loadChapter, studio, toon, touched } from '../studio.svelte.js';
	import { framePng } from '../frames.js';

	let variants = $state([]);
	let generated = $state([]);
	let over = $state(false);
	const id = $derived(studio.sel?.id);
	const v = $derived(studio.version);

	$effect(() => {
		void v;
		if (!id) return;
		api('GET', `/api/variants?toon=${toon()}&ch=${ch()}&id=${id}`).then((r) => (variants = r));
		api('GET', `/api/generated?ch=${ch()}&id=${id}`).then((r) => (generated = r));
	});

	async function pick(vid) {
		await api('POST', '/api/variants/pick', { toon: toon(), ch: ch(), id, vid });
		await loadChapter(true);
		await api('POST', '/api/frame', { toon: toon(), ch: ch(), id, png: await framePng() });
		await loadChapter(true);
		studio.message = `${vid} is the shot now.`;
	}
	function load(variant) {
		studio.sel.shot = { ...JSON.parse(JSON.stringify(variant.shot)), id };
		touched();
	}
	async function add(files) {
		for (const file of files) {
			const data = await new Promise((ok) => { const r = new FileReader(); r.onload = () => ok(r.result); r.readAsDataURL(file); });
			await api('POST', '/api/generated', { ch: ch(), id, name: file.name, data });
		}
		generated = await api('GET', `/api/generated?ch=${ch()}&id=${id}`);
	}
	async function approve(file) {
		const note = await askDialog({ title: 'The chosen image', message: 'Approves the shot with this image (review + dataset).', label: 'Note (optional)', confirmLabel: 'Approve', skipLabel: 'Approve without note' });
		if (note === null) return;
		const r = await api('POST', '/api/generated/approve', { toon: toon(), ch: ch(), id, file, note });
		studio.message = r.out;
		await loadChapter(true);
	}
</script>

{#if id}
	<div class="panel">
		<figure>
			<img src={`/out/${ch()}/frames/${id}.png?v=${v}`} alt="" />
			<figcaption>The saved frame</figcaption>
		</figure>
		<h3>Variants</h3>
		{#each variants as x (x.id)}
			<figure>
				<img src={`/out/${ch()}/variants/${id}--${x.id}.png?v=${v}`} alt="" />
				<div class="row">
					<button type="button" class="secondary" onclick={() => load(x)}>Load</button>
					<button type="button" onclick={() => pick(x.id)}>Take this</button>
				</div>
				<figcaption>{x.id} · {x.note || 'no note'}</figcaption>
			</figure>
		{:else}
			<p class="hint">None yet – change the shot, then "Keep as variant".</p>
		{/each}
		<h3>Generated</h3>
		<label class="drop" class:over ondragover={(e) => { e.preventDefault(); over = true; }} ondragleave={() => (over = false)} ondrop={(e) => { e.preventDefault(); over = false; add([...e.dataTransfer.files]); }}>
			Drop an image here (Luma, a photo, a drawing) or pick one
			<input type="file" accept="image/png,image/jpeg,image/webp" multiple onchange={(e) => add([...e.currentTarget.files])} />
		</label>
		{#each generated as f (f)}
			<figure>
				<img src={`/out/${ch()}/generated/${f}?v=${v}`} alt="" />
				<div class="row"><button type="button" onclick={() => approve(f)}>Approve as chosen</button></div>
				<figcaption>{f}</figcaption>
			</figure>
		{/each}
	</div>
{/if}

<style>
	.panel {
		padding: var(--space-1);
		display: grid;
		gap: var(--space-1);
	}
	h3 {
		margin: var(--space-05) 0 0;
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	figure {
		margin: 0;
		display: grid;
		gap: var(--space-05);
	}
	img {
		width: 100%;
		display: block;
		background: var(--ink);
	}
	figcaption,
	.hint {
		font-size: var(--type-caption-size);
		margin: 0;
	}
	.row {
		display: flex;
		gap: var(--space-05);
		justify-content: flex-end;
	}
	.drop {
		display: grid;
		gap: var(--space-05);
		padding: var(--space-1);
		border: var(--border-width-regular) dashed var(--ink);
		font-size: var(--type-caption-size);
	}
	.drop.over {
		background: var(--ink);
		color: var(--paper);
	}
</style>
