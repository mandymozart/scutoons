<script lang="ts">
	import { untrack } from 'svelte';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { PropRow, PropSelect, Segmented } from '@scutoons/ui';
	import {
		AUTHORED_VIEW,
		CHARACTER_IMAGE_VARIANTS,
		COLLAGE_LOOKS,
		DEFAULT_MOOD,
		defaultLookId,
		findLook,
		findPersona,
		lookMoods,
		moodLabel,
		toonAssetRef,
		type CharacterImageVariant,
		type CharacterRegistry,
		type CollagePiece,
		type MoodVocabulary,
		type Path,
		type PieceFigure
	} from '@scutoons/content';
	import { figurePath, pieceAt, setFigure, setPreset } from '$lib/editor/collage-edit.ts';
	import { figureImage, figureViews, imageRatio } from '$lib/editor/collage-pick.ts';
	import { lookLabel, pieceKindLabel, pieceLabel } from '$lib/editor/collage-words.ts';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { toast } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PlaceFields from '$lib/components/collage/place-fields.svelte';
	import LookFields from '$lib/components/collage/look-fields.svelte';

	/**
	 * The ONE inspector of a collage piece (research/COLLAGE.md §7), on a slice and on a score: its kind and name, a
	 * character picture's mood and view, the looks (a pick names the look and drops the piece's own values), and folded
	 * the fine-tuning – place, size, blend, fill, outline, shadow, blur (open when the piece has its own values). Like the
	 * text box inspector: compact property rows, drag a field's handle to change it, click to type. A re-click of the
	 * chosen view (no re-pick still decoding), or of the chosen look while the piece has no values of its own, changes
	 * nothing (no empty undo step, redo kept); with own values, a re-click of the chosen look takes the piece back to the
	 * look (drops them).
	 */
	let {
		doc,
		path,
		host,
		registry = null,
		moods,
		onremove,
		onpage
	}: {
		doc: ContentDoc;
		/** the piece (`[…slice, 'pieces', i]`, `[…, 'collage', 'pieces', i]`) */
		path: Path;
		host: 'slice' | 'score';
		registry?: CharacterRegistry | null;
		moods?: MoodVocabulary | null;
		onremove?: () => void;
		/** a score piece's Bar / Page switch, done by the host (place-fields.svelte) */
		onpage?: (page: boolean) => void;
	} = $props();

	const piece = $derived(doc.get(doc.primary, path) as CollagePiece | undefined);
	const key = (field: string) => `${path.join('.')}.${field}`;
	/** one edit of the piece = one undo step (the same key within ~1s merges: one scrub, one step) */
	function change(edit: (piece: CollagePiece) => void, field: string) {
		doc.edit((file) => {
			const target = pieceAt(file, path);
			if (target) edit(target);
		}, key(field));
	}

	// the fold opens on a piece with its own values – once per piece (a look picked later, an undo don't shut it)
	let advanced = $state(false);
	let foldedFor: string | undefined;
	$effect.pre(() => {
		const id = piece?.id;
		if (id === foldedFor) return;
		foldedFor = id;
		untrack(() => (advanced = !!piece?.style));
	});

	// a character picture: its look's moods, the view – a change re-resolves the file (and its ratio) in one edit
	const character = $derived(piece?.figure ? registry?.characters.find((c) => c.id === piece.figure?.id) : undefined);
	const moodIds = $derived.by(() => {
		const persona = findPersona(character, piece?.figure?.persona);
		return lookMoods(findLook(persona, piece?.figure?.look ?? defaultLookId(persona)), moods);
	});
	/**
	 * the views with a file of their own for the piece's mood (null while the character isn't known): any other is
	 * disabled – the fallback rule would draw it as the portrait, a figure readers never see
	 */
	const ownViews = $derived(character && piece?.figure ? figureViews(character, piece.figure, isKnownMissing) : null);
	/** the view as drawn: a stored view without a file of its own shows as the portrait it is */
	const shownView = $derived(piece?.figure && (!ownViews || ownViews.includes(piece.figure.view)) ? piece.figure.view : AUTHORED_VIEW);
	const viewMissing = (view: CharacterImageVariant, mood: string = DEFAULT_MOOD) =>
		m['creator.piece_view_missing']({ view: m[`creator.piece_view_${view}`](), mood: moodLabel(mood, textLanguage.names, moods) });
	/** the head's name – the one the lists and the canvas show (collage-words.ts `pieceLabel`) */
	const name = $derived(pieceLabel(piece, registry, moods));
	/**
	 * per piece: the newest re-pick still decoding, with every change asked for since the last applied one (`n` is
	 * unique, so an older decode never passes for a newer re-pick)
	 */
	const pending = new Map<string, { n: number; next: Partial<PieceFigure> }>();
	let asks = 0;
	async function refigure(next: Partial<PieceFigure>) {
		if (!piece?.figure) return;
		// the decode below can outlive the selection (the host reuses this inspector with a new path) and a Remove can
		// renumber the pieces: the edit goes to this piece by its id in this document, never to whatever `path` is then
		const { id } = piece;
		const at = [...path];
		const target = doc;
		// a mood and then a view picked while the first still decodes: the newer re-pick carries both
		const asked = { n: ++asks, next: { ...pending.get(id)?.next, ...next } };
		const figure: PieceFigure = { ...piece.figure, ...asked.next };
		// a view needs a file of its own (the row disables the others; a mood still decoding can take one away): a view
		// asked for without one changes nothing, a mood without the current view's file shows its portrait
		const own = figureViews(character, figure, isKnownMissing);
		if (own.length && !own.includes(figure.view)) {
			if (next.view) return void toast(viewMissing(next.view, figure.mood), { type: 'warning' });
			figure.view = own.includes(AUTHORED_VIEW) ? AUTHORED_VIEW : own[0];
		}
		// no picture at all (or only a known-missing one): the piece keeps its own – the "?" substitute is no art
		const root = figureImage(character, figure, isKnownMissing);
		if (!root) return void toast(m['creator.collage_no_picture']({ name: shownCharacterName(character, figure.persona) || figure.id }), { type: 'warning' });
		pending.set(id, asked);
		const ratio = await imageRatio(asset(root));
		if (pending.get(id)?.n !== asked.n) return;
		pending.delete(id);
		const now = figurePath(target.get(target.primary, []), at, id);
		if (!now) return;
		// the host's path form: a toon document's own (`../characters/…`), a score's (toon '') from the root
		const src = toonAssetRef(target.toon, root);
		target.edit((file) => {
			const p = pieceAt(file, now);
			if (p?.id === id) setFigure(p, figure, src, ratio);
		}, `${now.join('.')}.figure`);
	}
</script>

{#if piece}
	<div class="inspector" data-testid="piece-inspector" data-piece={piece.id}>
		<div class="row head">
			<h3>{pieceKindLabel(piece)} · {name}</h3>
			{#if onremove}<button class="secondary" onclick={onremove} data-testid="piece-remove">{m['creator.remove']()}</button>{/if}
		</div>

		{#if piece.figure}
			<PropRow label={m['creator.piece_picture']()} testid="piece-picture">
				<PropSelect
					icon="smile"
					label={m['creator.piece_mood']()}
					required
					value={piece.figure.mood ?? DEFAULT_MOOD}
					options={moodIds.map((id) => ({ value: id, label: moodLabel(id, textLanguage.names, moods) }))}
					onchange={(mood) => mood && refigure({ mood })}
					testid="piece-mood"
				/>
				<span data-prop-line>
					<Segmented
						label={m['creator.piece_view']()}
						stretch
						radio
						value={shownView}
						onchange={(view) => (view !== piece.figure?.view || pending.has(piece.id)) && refigure({ view })}
						options={CHARACTER_IMAGE_VARIANTS.map((v) => {
							const disabled = !!ownViews && !ownViews.includes(v);
							return { value: v, label: m[`creator.piece_view_${v}`](), disabled, title: disabled ? viewMissing(v, piece.figure?.mood) : undefined, testid: `piece-view-${v}` };
						})}
						testid="piece-view"
					/>
				</span>
			</PropRow>
		{/if}

		<!-- the looks (code presets – COLLAGE.md §2): a pick names it and drops the piece's own values; the fold fine-tunes -->
		<Segmented
			grid
			label={m['creator.piece_look']()}
			value={piece.preset ?? ''}
			onchange={(look) => ((look || undefined) !== piece.preset || piece.style) && change((target) => setPreset(target, look || undefined), 'preset')}
			options={[{ value: '', label: m['creator.look_none'](), testid: 'piece-look-none' }, ...COLLAGE_LOOKS.map((l) => ({ value: l.id, label: lookLabel(l.id), testid: `piece-look-${l.id}` }))]}
			testid="piece-looks"
		/>

		<details class="advanced" bind:open={advanced} data-testid="piece-advanced">
			<summary>{m['creator.advanced']()}</summary>
			<div class="props">
				<PlaceFields {doc} {path} {piece} {host} {onpage} />
				<LookFields {piece} {change} />
			</div>
		</details>
	</div>
{/if}

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-05);
	}
	h3 {
		margin: 0;
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.props {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.advanced[open] {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
</style>
