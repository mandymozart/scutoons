<script lang="ts">
	import {
		matchCharacterImageFile,
		newLook as lookEntry,
		setLookImage,
		type AssetRequests,
		type CharacterFileSlot,
		type MoodVocabulary,
		type RegistryCharacter
	} from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { edit, editCharacter, ensureMoods } from '$lib/editor/characters.ts';
	import { applyApose, closeRequests } from '$lib/editor/character-form.ts';
	import { toWebp, uploadOrReplace } from '@scutoons/media';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Import images …" (owner 2026-10-03): each file whose name says its slot (`matchCharacterImageFile`: expressions of
	 * any persona / look, A-pose looks, transitions) goes there as WebP at its own size (no density variants) under the
	 * convention path; a file already there gets the new bytes (a new version, the CDN purged). New looks and moods are
	 * added, answered requests closed – one undo step. Names for another character, a persona the character lacks or a
	 * transition between looks it lacks are reported, not guessed. `bust` is set afterwards (previews load again).
	 */
	let {
		registry,
		moodsDoc,
		requestsDoc,
		moods,
		requests,
		character,
		languages,
		bust = $bindable(0)
	}: {
		registry: ContentDoc;
		moodsDoc: ContentDoc;
		requestsDoc: ContentDoc;
		moods: MoodVocabulary;
		requests: AssetRequests | null;
		character: RegistryCharacter;
		languages: string[];
		bust?: number;
	} = $props();

	const id = $derived(character.id);
	let importInput = $state<HTMLInputElement>();
	let importing = $state<{ done: number; total: number } | null>(null);
	let summary = $state('');
	let problems = $state<string[]>([]);

	function problemOf(slot: Extract<CharacterFileSlot, { kind: 'unknown' }>): string {
		if (slot.reason === 'other-character') return m['creator.characters_import_other']({ name: slot.name ?? '' });
		if (slot.reason === 'unknown-persona') return m['creator.characters_import_unknown_persona']({ name: slot.name ?? '' });
		if (slot.reason === 'unknown-look') return m['creator.characters_import_unknown_look']({ name: slot.name ?? '' });
		return m['creator.characters_import_no_slot']({ example: `${id}_portrait-neutral.png, ${id}_apose-swim.png` });
	}

	async function importImages(event: Event) {
		const files = [...((event.currentTarget as HTMLInputElement).files ?? [])];
		if (importInput) importInput.value = '';
		if (!files.length) return;
		problems = [];
		summary = '';
		const counts = { uploaded: 0, replaced: 0, unchanged: 0 };
		const done: Exclude<CharacterFileSlot, { kind: 'unknown' }>[] = [];
		importing = { done: 0, total: files.length };
		// transitions last, matched against the looks this import adds (`boy_helmet_portrait-…` + `boy_transition-helmet-to-…`)
		const known = { id: character.id, personas: character.personas.map((p) => ({ ...p, looks: [...p.looks] })) };
		const isTransition = (f: File) => /_transition-/i.test(f.name);
		for (const file of [...files.filter((f) => !isTransition(f)), ...files.filter(isTransition)]) {
			const slot = matchCharacterImageFile(file.name, known);
			if (slot.kind === 'expression' && slot.newLook) known.personas.find((p) => p.id === slot.persona)?.looks.push(lookEntry(slot.look, undefined, false));
			try {
				if (slot.kind === 'unknown') throw new Error(problemOf(slot));
				const bitmap = await createImageBitmap(file);
				let blob: Blob;
				try {
					blob = (await toWebp(bitmap)).blob;
				} finally {
					bitmap.close();
				}
				counts[await uploadOrReplace(blob, slot.path, 'character import')]++;
				done.push(slot);
			} catch (e) {
				problems = [...problems, `${file.name}: ${e instanceof Error ? e.message : String(e)}`];
			}
			importing = { done: importing.done + 1, total: files.length };
		}
		if (done.length) {
			const newMoods = [...new Set(done.flatMap((s) => (s.kind === 'expression' && !moods.moods.some((x) => x.id === s.mood) ? [s.mood] : [])))];
			if (newMoods.length) {
				ensureMoods(moodsDoc);
				edit<MoodVocabulary>(moodsDoc, (v) => {
					for (const mood of newMoods) v.moods.push({ id: mood, label: Object.fromEntries(languages.map((l, i) => [l, i === 0 ? mood : ''])), order: Math.max(0, ...v.moods.map((x) => x.order ?? 0)) + 1 });
				});
			}
			editCharacter(registry, id, (c: RegistryCharacter) => {
				for (const s of done) {
					if (s.kind === 'apose') {
						applyApose(c, s.look, s.path, languages);
						continue;
					}
					const p = c.personas.find((x) => x.id === s.persona);
					if (!p) continue;
					if (s.kind === 'expression') {
						// (push, then read back: the edit works on reactive proxies – a pushed object is not the stored one)
						if (!p.looks.some((x) => x.id === s.look)) p.looks.push(lookEntry(s.look, { [languages[0] ?? 'en']: s.look }, !p.looks.length));
						setLookImage(p.looks.find((x) => x.id === s.look)!, s.mood, s.view, s.path);
					} else {
						const t = p.transitions?.find((x) => x.from === s.from && x.to === s.to);
						if (t) t.image = s.path;
						else {
							p.transitions ??= [];
							p.transitions.push({ from: s.from, to: s.to, image: s.path });
						}
					}
				}
			});
			closeRequests(requestsDoc, requests, id, done);
		}
		importing = null;
		summary = m['creator.characters_import_summary'](counts);
		bust = Date.now();
	}
</script>

<div class="import">
	<button type="button" class="secondary" disabled={!!importing} onclick={() => importInput?.click()} title={m['creator.characters_import_hint']({ id })} data-testid="character-import">
		{importing ? m['creator.import_slices_progress']({ done: importing.done, total: importing.total }) : m['creator.characters_import']()}
	</button>
	<input bind:this={importInput} type="file" accept="image/png,image/jpeg,image/webp" multiple hidden onchange={importImages} />
	{#if summary}<span class="sc-caption" role="status">{summary}</span>{/if}
</div>
{#each problems as problem}<p class="error" role="alert">{problem}</p>{/each}

<style>
	.import {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		align-items: center;
		margin-bottom: 1rem;
	}
</style>
