# @scutoons/reader

The reader's rendering, shared by the viewer (reading) and the creator (WYSIWYG canvas, draft preview) so a slice
looks the same in both: the slice image with its live text boxes, and the onomatopoeia layer of the visual novel.
Also the pure slice actions (audio events, VN triggers) with their app wiring injected.

Rules (agents/PLATFORM.md dogmas): no app state, no routing, no paraglide. Components take props and callbacks; the
app passes resolved URLs, the text language and its own interactive layers. Depends on `@scutoons/content` (types,
helpers) and `svelte` only.

## Public API (`import … from '@scutoons/reader'`)

| Export | What |
|---|---|
| `Slice` | `.slice` box (`container-type: inline-size`, so text sizes are `cqw`): `<img>` + `Dialogs`. Props: `slice`, `src` (resolved image URL), `lang`, `alt` (default `Slice`), snippets `before` (app layers under the image, e.g. samples / VN trigger hotzones) and `children` (after the text boxes); other attributes and `{@attach}` go to the `.slice` element. |
| `Dialogs` | text-box layer (`position:absolute; inset:0`) for `slice.dialog`. Props: `dialogs`, `lang`. Must sit inside an element with `container-type: inline-size` and the class `slice` (the auto-fit pass observes `.slice`). |
| `Dialog` | one text box (variants bubble / caption / narration / sfx, auto-fit down to 80 %). |
| `Onomatopoeia` | one onomatopoeia (variants default / outline / bubble / thorns, effects none / shiver / pulse / wobble). Prop `onomatopoeia: ActiveOnomatopoeia`; calls `onomatopoeia.onComplete` after delay + duration. |
| `sliceAudio` | action: music volumes while the slice is in view + its sample trigger. Config `{ events, index, slices, player }`, `player: SliceAudioPlayer \| null` = `{ triggerSample, getTrackIds, updateMusicVolumes }` from the app's audio engine. |
| `sliceEvents` | action: slice-triggered VN events (click / hover / intersect, optional rect, `once`). Config `{ events, run(event) → url?, onNavigate? }`; the VN logic stays in the app (`run`). Not used by the viewer today (its VN triggers are components). |
| `getMusicStateForSlice(index, slices, trackIds)` | last known volume per track, searched backwards from a slice. |
| `isWithinRect(event, element, rect?)` | pointer inside a % rect of an element. |
| types | `ActiveOnomatopoeia`, `PercentRect`, `SliceAudioConfig`, `SliceAudioPlayer`, `SliceEventsConfig` |

## Example

```svelte
<script lang="ts">
	import { fromAction } from 'svelte/attachments';
	import { Slice, sliceAudio } from '@scutoons/reader';
	import type { Slice as SliceModel } from '@scutoons/content';

	let { slice, index, slices, player }: { slice: SliceModel; index: number; slices: SliceModel[]; player: any } =
		$props();
</script>

<Slice
	{slice}
	src={`https://cdn.scutoons.com/catpope/${slice.image}`}
	lang="de"
	{@attach fromAction(sliceAudio, () => ({ events: slice.audio, index, slices, player }))}
>
	{#snippet before()}
		<!-- app layers: sample hotzones, VN trigger hotzones -->
	{/snippet}
</Slice>
```

Text boxes only (the creator canvas draws its own image and edit overlay):

```svelte
<div class="slice" style="position:relative; container-type:inline-size">
	<img src={url} alt="" />
	<Dialogs dialogs={slice.dialog} lang="en" />
</div>
```

Fonts and colours: the dialogs use `IBM Plex Mono` and `Bangers` (`font: 'lettering'`) from `@scutoons/tokens`; import
the tokens in the app.

## Check

`npm run check -w @scutoons/reader` = `svelte-check --fail-on-warnings`.
