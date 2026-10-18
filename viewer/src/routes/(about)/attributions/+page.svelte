<script lang="ts">
	/**
	 * The app's own credits (owner 2026-10-18): the icons, fonts, libraries and models SCUTOONS ships, with what their
	 * licences ask for – not the story, character or song credits (those are on each toon's page). The headings and the
	 * intro are messages; the credit lines stay in their original English, as licences are. A link per name (source)
	 * and per licence. Linked from the landing page's footer.
	 */
	import { m } from '$lib/paraglide/messages.js';

	type Part = string | { text: string; href: string };
	type Group = { title: () => string; note?: () => string; lines: Part[][] };

	const OFL = { text: 'SIL OFL 1.1', href: 'https://openfontlicense.org/open-font-license-official-text/' };
	const MIT = { text: 'MIT', href: 'https://opensource.org/license/mit' };
	const APACHE = { text: 'Apache-2.0', href: 'https://www.apache.org/licenses/LICENSE-2.0' };
	const CC_BY_4 = { text: 'CC BY 4.0', href: 'https://creativecommons.org/licenses/by/4.0/' };
	const VEROVIO = '6.3.0'; // package-lock.json – change with the dependency

	const GROUPS: Group[] = [
		{
			title: () => m['attributions_page.icons'](),
			lines: [
				[
					{ text: 'Teddy', href: 'https://thenounproject.com/icon/1532101/' },
					' by Vicons Design from Noun Project (',
					{ text: 'CC BY 3.0', href: 'https://creativecommons.org/licenses/by/3.0/' },
					')'
				]
			]
		},
		{
			title: () => m['attributions_page.fonts'](),
			lines: [
				[{ text: 'IBM Plex Mono', href: 'https://github.com/IBM/plex' }, ' – © 2017 IBM Corp. – ', OFL],
				[{ text: 'Bangers', href: 'https://github.com/googlefonts/bangers' }, ' – © 2010 The Bangers Project Authors – ', OFL]
			]
		},
		{
			title: () => m['attributions_page.music'](),
			lines: [
				[
					{ text: `Verovio ${VEROVIO}`, href: 'https://www.verovio.org' },
					' – © Laurent Pugin, Rodolfo Zitellini and contributors (RISM Digital Center) – LGPL-3.0-or-later (',
					{ text: 'LGPL-3.0', href: 'https://www.gnu.org/licenses/lgpl-3.0.html' },
					', ',
					{ text: 'GPL-3.0', href: 'https://www.gnu.org/licenses/gpl-3.0.html' },
					'), used unmodified – ',
					{ text: 'source of this release', href: `https://github.com/rism-digital/verovio/tree/version-${VEROVIO}` }
				],
				[
					'Its music fonts ',
					{ text: 'Leipzig', href: 'https://github.com/rism-digital/leipzig' },
					' and ',
					{ text: 'Gootville', href: `https://github.com/rism-digital/verovio/tree/version-${VEROVIO}/fonts/Gootville` },
					' (Grzegorz Pruchniakowski) – ',
					OFL
				]
			]
		},
		{
			title: () => m['attributions_page.camera'](),
			lines: [
				[{ text: 'TensorFlow.js', href: 'https://github.com/tensorflow/tfjs' }, ' – © Google LLC – ', APACHE],
				[{ text: '@tensorflow-models/mobilenet', href: 'https://github.com/tensorflow/tfjs-models/tree/master/mobilenet' }, ' – © Google LLC – ', APACHE],
				[{ text: 'MobileNet V2 (1.0, 224) weights', href: 'https://www.kaggle.com/models/google/mobilenet-v2' }, ' – Google – ', APACHE],
				[{ text: 'jsQR', href: 'https://github.com/cozmo/jsQR' }, ' – Cosmo Wolfe – ', APACHE]
			]
		},
		{
			title: () => m['attributions_page.three_d'](),
			lines: [[{ text: 'three.js', href: 'https://threejs.org' }, ' – © 2010–2026 three.js authors – ', MIT]]
		},
		{
			title: () => m['attributions_page.audio'](),
			lines: [[{ text: 'howler.js', href: 'https://howlerjs.com' }, ' – © 2013–2020 James Simpson and GoldFire Studios, Inc. – ', MIT]]
		},
		{
			title: () => m['attributions_page.creator_admin'](),
			note: () => m['attributions_page.creator_admin_note'](),
			lines: [
				[
					{ text: 'LAME MP3 encoder', href: 'https://lame.sourceforge.io' },
					' – The LAME Project – ',
					{ text: 'LGPL-2.0-or-later', href: 'https://www.gnu.org/licenses/old-licenses/lgpl-2.0.html' },
					', inside ',
					{ text: 'wasm-media-encoders', href: 'https://github.com/arseneyr/wasm-media-encoders' },
					' (© arseneyr – ',
					MIT,
					')'
				],
				[
					{ text: 'libvorbis and libogg', href: 'https://xiph.org/vorbis/' },
					' – © Xiph.Org Foundation – ',
					{ text: 'BSD-3-Clause', href: 'https://opensource.org/license/bsd-3-clause' },
					', inside wasm-media-encoders'
				],
				[
					{ text: 'Fox', href: 'https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/Fox' },
					' – model by PixelMannen (',
					{ text: 'CC0', href: 'https://creativecommons.org/publicdomain/zero/1.0/' },
					'), rigging & animation by tomkranis (',
					CC_BY_4,
					'), glTF by @AsoboStudio and @scurest (',
					CC_BY_4,
					') – ',
					{ text: 'Khronos glTF Sample Assets', href: 'https://github.com/KhronosGroup/glTF-Sample-Assets' }
				],
				[
					{ text: 'X Bot', href: 'https://www.mixamo.com' },
					' – Mixamo (Adobe), via the ',
					{ text: 'three.js examples', href: 'https://github.com/mrdoob/three.js/blob/dev/examples/models/gltf/Xbot.glb' },
					' – used internally as a stand-in figure, under the ',
					{ text: 'Mixamo terms', href: 'https://helpx.adobe.com/creative-cloud/faq/mixamo-faq.html' }
				]
			]
		},
		{
			title: () => m['attributions_page.other'](),
			lines: [
				[
					{ text: 'Svelte', href: 'https://svelte.dev' },
					', ',
					{ text: 'SvelteKit', href: 'https://svelte.dev/docs/kit' },
					', ',
					{ text: 'Paraglide', href: 'https://inlang.com/m/gerre34r/library-inlang-paraglideJs' },
					', ',
					{ text: 'runed', href: 'https://runed.dev' },
					', ',
					{ text: 'Feather icons', href: 'https://feathericons.com' },
					' and others – MIT / ISC / BSD licences'
				]
			]
		}
	];
</script>

<svelte:head><title>{m.attributions()} · SCUTOONS</title></svelte:head>

<div class="page" data-testid="attributions">
	<h1>{m.attributions()}</h1>

	<p class="intro">{m['attributions_page.intro']()}</p>

	{#each GROUPS as group (group)}
		<section>
			<h2>{group.title()}</h2>
			{#if group.note}<p class="note">{group.note()}</p>{/if}
			<ul>
				{#each group.lines as line, i (i)}
					<li>
						{#each line as part, j (j)}{#if typeof part === 'string'}{part}{:else}<a href={part.href} target="_blank" rel="noopener">{part.text}</a>{/if}{/each}
					</li>
				{/each}
			</ul>
		</section>
	{/each}
</div>

<style>
	.page {
		padding: 2rem;
		max-width: 50rem;
		margin: 0 auto;
	}

	h1 {
		margin-bottom: 1rem;
	}

	section {
		margin: 2rem 0;
	}

	h2 {
		margin-bottom: 1rem;
	}

	.note {
		margin: 0 0 0.5rem 1rem;
		font-size: 0.9rem;
		opacity: 0.8;
	}

	ul {
		margin: 0 0 0 1rem;
		padding: 0;
		list-style: none;
	}

	li {
		margin-top: 0.5rem;
		font-size: 0.9rem;
		overflow-wrap: anywhere;
	}

	a {
		color: inherit;
		text-decoration: underline;
	}
</style>
