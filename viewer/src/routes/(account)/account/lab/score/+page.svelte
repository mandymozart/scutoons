<script lang="ts">
	// Dev-only draft of the score view (research/SCORES.md §4, FEATURES 10.97): a MusicXML file from the local CDN
	// (`media/scutoons/scores/`, a dev copy) in the shared ScoreView – before scores exist as catalog items. Also the
	// lettered cover (§1) and the DIY book's ageing (§2a) at a chosen age; the looks (score-looks.ts – Gootville, colours,
	// boiling lines, hopping notes) and the medieval one (owner 2026-10-17: illuminated frames with the characters, a
	// medieval face for the title – the trial fonts are ./fonts, Google Fonts, OFL; this page only).
	import { ageLook } from '@scutoons/content';
	import { CoverLettering, SCORE_LOOKS } from '@scutoons/reader';
	import { ScoreCollage, ScoreIllumination, ScoreView } from '@scutoons/reader/score';
	import { ScoreRiot } from '@scutoons/reader/score-lab';
	import { Segmented } from '@scutoons/ui';
	import { asset } from '$lib/utils/cdn.ts';
	import './fonts/lab-fonts.css';

	const isDev = import.meta.env.DEV;
	const SCORES = [
		{ id: 'cloud9-en', title: 'Cloud 9' },
		{ id: 'likeababy-en', title: 'Like A Baby' },
		{ id: 'nohesaid-en', title: 'No He Said' },
		{ id: 'notmycity-en', title: 'Not My City' },
		{ id: 'oncetwicethrice-en', title: 'Once, Twice, Thrice' },
		{ id: 'roarlittlekitty-en', title: 'Roar Little Kitty' }
	];
	const DIY_AGES = { tint_days: 14, spots_days: 30, full_days: 90 };
	/** `?look=` picks one (default the newest trial); `?score=` the open score (`none`: the page); `?part=full` the full score
	 *  instead of the choir part readers get (score-actions.svelte, `readerPart`) */
	const query = typeof location === 'undefined' ? null : new URLSearchParams(location.search);
	const file = query?.get('part') === 'full' ? '' : '-choir';
	let open = $state<(typeof SCORES)[number] | null>(query?.get('score') === 'none' ? null : (SCORES.find((s) => s.id === query?.get('score')) ?? SCORES[0]));
	/** the DIY book's age in days (0 = new) */
	let days = $state(0);
	let lookId = $state<string>(query?.get('look') ?? 'collage');
	const scoreLook = $derived(SCORE_LOOKS.find((l) => l.id === lookId) ?? null);
	const LOOK_OPTIONS = [
		{ value: 'plain', label: 'Plain' },
		{ value: 'old-school', label: 'Old school' },
		{ value: 'old-school-colour', label: 'Old school colour' },
		{ value: 'wild', label: '90s wild' },
		{ value: 'collage', label: 'Collage' }
	];
	/** the trial faces for the medieval title (and the initial's letter) */
	const TITLE_FONTS = ['UnifrakturMaguntia', 'UnifrakturCook', 'Grenze Gotisch', 'Pirata One', 'Texturina', 'Uncial Antiqua', 'MedievalSharp', 'Jacquard 24'];
	/** lyrics: Verovio spaces the words by Times' widths – a face much wider runs them together */
	const LYRIC_FONTS = ['Times New Roman', 'IM Fell English', 'Almendra'];
	let titleFont = $state(TITLE_FONTS[0]);
	let lyricFont = $state(LYRIC_FONTS[0]);
	// the view is portaled to <body>: the faces go on the root
	$effect(() => {
		const root = document.documentElement.style;
		root.setProperty('--score-title-font', `'${titleFont}'`);
		root.setProperty('--score-lyric-font', `'${lyricFont}'`);
		return () => {
			root.removeProperty('--score-title-font');
			root.removeProperty('--score-lyric-font');
		};
	});
	const C = (name: string, file: string) => asset(`characters/${name}/${file}`);
	const PORTRAITS = [
		C('catpope', 'catpope_portrait-happy.webp'),
		C('billthefrog', 'billthefrog_portrait-happy.webp'),
		C('yellowmouse', 'yellowmouse_portrait-happy.webp'),
		C('crows', 'crows_portrait-happy.webp'),
		C('catpope-human', 'catpope-human_portrait-happy.webp'),
		C('toptoy', 'toptoy_portrait-neutral.webp')
	];
	/** the collage's people (owner 2026-10-17: "the cat pope personas and moods … also the crows"): every persona in every
	 *  mood as avatar, portrait and full figure – a missing combination hides itself */
	const MOODS = ['acceptance', 'angry', 'bargaining', 'confused', 'crying', 'denial', 'disgusted', 'dizzy', 'embarrassed', 'furious', 'happy', 'laughing', 'neutral', 'sad', 'scared', 'smug', 'surprised', 'thinking'];
	const PERSONAS: [string, string, string[]][] = [
		['catpope', 'catpope', ['avatar', 'portrait', 'full']],
		['catpope-human', 'catpope-human', ['avatar', 'portrait', 'full']],
		['catpope-biker', 'catpope-biker', ['avatar', 'portrait']],
		['catpope-biker', 'catpope-biker_helmet', ['avatar', 'portrait']],
		['crows', 'crows', ['avatar', 'portrait', 'full']]
	];
	const PICTURES = [
		...PERSONAS.flatMap(([dir, name, views]) => views.flatMap((view) => MOODS.map((mood) => C(dir, `${name}_${view}-${mood}.webp`)))),
		C('catpope-cat', 'catpope-cat_portrait-neutral.webp'),
		C('catpope-mountaintiger', 'catpope-mountaintiger_full-neutral.webp'),
		C('catpope-mountaintiger', 'catpope-mountaintiger_portrait-neutral.webp')
	];
	const FIGURES = ['catpope', 'billthefrog', 'boy', 'toptoy', 'yellowbubu', 'crows', 'passerby', 'catpope-mountaintiger', 'catpope-human'].map((c) =>
		C(c, `${c}_full-neutral.webp`)
	);
	const look = $derived(ageLook(DIY_AGES, 0, days * 86_400_000));
</script>

{#if isDev}
	<div class="view lab">
		<h1>Score view (draft)</h1>
		<p>
			<label>DIY book age: {days} days <input type="range" min="0" max="120" bind:value={days} data-testid="score-age-days" /></label>
		</p>
		<p><Segmented label="Look" options={LOOK_OPTIONS} value={lookId} onchange={(v) => (lookId = v)} testid="score-look" /></p>
		{#if lookId === 'medieval' || lookId === 'wild'}
			<p class="fonts">
				<label>Lyrics <select bind:value={lyricFont} data-testid="score-lyric-font">{#each LYRIC_FONTS as f (f)}<option value={f}>{f}</option>{/each}</select></label>
			</p>
			<div class="faces" role="radiogroup" aria-label="Title face">
				{#each TITLE_FONTS as f (f)}
					<button type="button" role="radio" aria-checked={titleFont === f} class="face" onclick={() => (titleFont = f)} data-testid="score-title-font">
						<span class="sample" style:font-family="'{f}'">{open?.title ?? 'Roar Little Kitty'}</span>
						<span class="name">{f}</span>
					</button>
				{/each}
			</div>
		{/if}
		<div class="covers">
			{#each SCORES as score (score.id)}
				<button class="cover" onclick={() => (open = score)} data-testid="score-open" title={score.title}>
					<CoverLettering text={score.title} />
				</button>
			{/each}
		</div>
	</div>
	{#if open}
		{#key open.id}
			<ScoreView
				src={asset(`scutoons/scores/${open.id}${file}.musicxml`)}
				pdf={asset(`scutoons/scores/${open.id}${file}.pdf`)}
				title={open.title}
				age={days > 0 ? { ...look, seed: `lab-${open.id}` } : null}
				look={scoreLook}
				decor={lookId === 'collage' ? riot : lookId === 'medieval' ? illuminate : lookId === 'wild' ? collage : undefined}
				onclose={() => (open = null)}
			/>
		{/key}
	{/if}
{/if}

{#snippet riot(page: number)}
	<ScoreRiot {page} pictures={PICTURES} seed="lab-{open?.id}" />
{/snippet}

{#snippet collage(page: number)}
	<ScoreCollage {page} portraits={PORTRAITS} figures={FIGURES} seed="lab-{open?.id}" />
{/snippet}

{#snippet illuminate(page: number)}
	<ScoreIllumination {page} title={open?.title ?? ''} portraits={PORTRAITS} figures={FIGURES} seed="lab-{open?.id}" />
{/snippet}

<style>
	.lab {
		padding: var(--space-1);
	}
	.fonts {
		display: flex;
		gap: var(--space-1);
	}
	.faces {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
		gap: var(--space-05);
		margin-bottom: var(--space-1);
	}
	.face {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-025);
		padding: var(--space-05) var(--space-075);
		background: #efe0bd;
		color: #a8321e;
		border: var(--border-width-regular) solid transparent;
		border-radius: var(--radius-none);
		cursor: pointer;
		text-align: left;
	}
	.face[aria-checked='true'] {
		border-color: var(--ink);
	}
	.sample {
		font-size: 2rem;
		line-height: 1.1;
	}
	.name {
		font-size: 0.75rem;
		color: var(--ink);
	}
	.covers {
		display: flex;
		gap: var(--space-1);
	}
	/* a stand-in for the shared blank songsheet icon (the owner's art comes later) */
	.cover {
		position: relative;
		width: 8rem;
		aspect-ratio: 3 / 4;
		background: repeating-linear-gradient(transparent 0 0.9rem, rgb(0 0 0 / 0.15) 0.9rem calc(0.9rem + 1px)), var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		cursor: pointer;
	}
</style>
