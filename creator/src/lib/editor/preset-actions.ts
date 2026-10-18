// The preset line's actions (agents/research/COLLAGE.md §12.4): Save as preset …, Update preset, ••• Rename … /
// Delete … – each behind its in-page dialog, then edits of two documents: the presets document `colorations` and the
// page's document that holds the piece. Ctrl+Z reaches only the page's document; the toast's Undo takes back both.
// The piece is found by its id every time (collage-edit.ts `piecePath`): a dialog or a toast outlives the selection.
import { COLLAGE_LOOKS, customPreset, type CollagePiece, type Colorations, type Path, type PieceStyle } from '@scutoons/content';
import { askDialog, choiceDialog, confirmDialog } from '@scutoons/ui';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { confirmDelete } from '$lib/editor/ask.ts';
import { pieceAt, piecePath, setPreset } from '$lib/editor/collage-edit.ts';
import {
	addPreset,
	dropPreset,
	pieceLook,
	placeholderName,
	presetIdFor,
	presetName,
	presetValues,
	removePreset,
	renamePreset,
	restorePreset,
	setPieceLook,
	styleOfPreset,
	updatePreset
} from '$lib/editor/collage-presets.ts';
import { lookLabel } from '$lib/editor/collage-words.ts';
import { ensurePresets, ownPresets } from '$lib/editor/presets-doc.ts';
import { toast } from '$lib/toasts.ts';
import { m } from '$lib/paraglide/messages.js';

/** where the piece is: the page's document, the piece's path in it, the presets document */
export interface PresetHost {
	doc: ContentDoc;
	path: Path;
	presets: ContentDoc;
}

/** the names the grid shows besides the custom presets: As drawn and the ten (a new preset's name never repeats them) */
const codeNames = () => [lookLabel('none'), ...COLLAGE_LOOKS.map((look) => lookLabel(look.id))];

/** an edit of the piece `id` in the page's document, wherever it is now (gone: nothing) */
function editPiece(doc: ContentDoc, path: Path, id: string, change: (piece: CollagePiece) => void): void {
	const at = piecePath(doc.get(doc.primary, []), path, id);
	if (!at) return;
	doc.edit((file) => {
		const piece = pieceAt(file, at);
		if (piece?.id === id) change(piece);
	});
}
const editPresets = (presets: ContentDoc, change: (file: Colorations) => void) => presets.edit((file) => change(file as Colorations));
const undo = (run: () => void) => ({ action: { label: m['creator.undo'](), run } });

/** the custom preset the piece names while it is live (not deleted) – what Update and ••• work on */
export function livePreset(piece: Pick<CollagePiece, 'preset'>, presets: ContentDoc) {
	const preset = customPreset(piece.preset, ownPresets(presets));
	return preset && preset.removed !== true ? { id: preset.id, name: lookLabel(preset.id, ownPresets(presets)) } : undefined;
}

/**
 * Save as preset …: asks for a name ("Preset n" when left empty), adds a preset with the piece's drawn values and lets
 * the piece name it instead of its own values – its pixels don't change. Undo: the preset goes, the piece as it was.
 */
export async function savePreset({ doc, path, presets }: PresetHost, piece: CollagePiece): Promise<void> {
	const at = [...path];
	const { id: pieceId } = piece;
	const values = presetValues(piece, ownPresets(presets));
	const was = pieceLook(piece);
	const fallback = presetName('', placeholderName(ownPresets(presets), (n) => m['creator.preset_placeholder']({ n: String(n) })), ownPresets(presets), { also: codeNames() });
	const typed = await askDialog({ title: m['creator.preset_save_title'](), label: m['creator.preset_name'](), placeholder: fallback, confirmLabel: m['creator.save_short']() });
	if (typed === null) return;
	const stored = ownPresets(presets);
	const name = presetName(typed, fallback, stored, { also: codeNames() });
	const id = presetIdFor(name, stored);
	// the first preset starts the document; its Undo then leaves no document behind (nothing to save or publish)
	const created = !presets.present.length;
	ensurePresets(presets);
	editPresets(presets, (file) => addPreset(file, { id, name, ...values }));
	editPiece(doc, at, pieceId, (p) => setPreset(p, id));
	toast(
		m['creator.preset_saved']({ name }),
		undo(() => {
			if (created && !presets.stored.length && ownPresets(presets).length === 1) presets.data[presets.primary] = null;
			else editPresets(presets, (file) => void dropPreset(file, id));
			editPiece(doc, at, pieceId, (p) => setPieceLook(p, was));
		})
	);
}

/**
 * Update preset: after a confirmation (every piece with it changes once published), the preset takes the piece's drawn
 * values and the piece names it without own values. Undo: the preset's old values, the piece as it was.
 */
export async function refreshPreset({ doc, path, presets }: PresetHost, piece: CollagePiece): Promise<void> {
	const preset = livePreset(piece, presets);
	if (!preset) return;
	const at = [...path];
	const { id: pieceId } = piece;
	const values = presetValues(piece, ownPresets(presets));
	const was = pieceLook(piece);
	const asked = { title: m['creator.preset_update_title']({ name: preset.name }), message: m['creator.preset_update_confirm'](), confirmLabel: m['creator.preset_update_label']() };
	if (!(await confirmDialog(asked))) return;
	let old: PieceStyle | undefined;
	editPresets(presets, (file) => {
		old = styleOfPreset(file, preset.id);
		updatePreset(file, preset.id, values);
	});
	if (!old) return;
	editPiece(doc, at, pieceId, (p) => setPreset(p, preset.id));
	const before = old;
	toast(
		m['creator.preset_updated']({ name: preset.name }),
		undo(() => {
			editPresets(presets, (file) => void updatePreset(file, preset.id, before));
			editPiece(doc, at, pieceId, (p) => setPieceLook(p, was));
		})
	);
}

/** •••: Rename … / Delete … the preset the piece names (a choice dialog titled with its name) */
export async function presetMenu({ presets }: PresetHost, piece: CollagePiece): Promise<void> {
	const preset = livePreset(piece, presets);
	if (!preset) return;
	const choices = [
		{ id: 'rename', label: m['creator.preset_rename']() },
		{ id: 'delete', label: m['creator.preset_delete']() }
	];
	const pick = await choiceDialog({ title: preset.name, choices });
	if (pick === 'rename') await rename(presets, preset);
	else if (pick === 'delete') await remove(presets, preset);
}

/** a new name (the same rules as Save; empty or unchanged = nothing) – the name only, its id stays, no toast */
async function rename(presets: ContentDoc, preset: { id: string; name: string }): Promise<void> {
	const typed = await askDialog({ title: m['creator.preset_rename_title'](), label: m['creator.preset_name'](), value: preset.name, confirmLabel: m['creator.preset_rename_label']() });
	if (!typed?.trim()) return;
	const name = presetName(typed, preset.name, ownPresets(presets), { except: preset.id, also: codeNames() });
	if (name !== preset.name) editPresets(presets, (file) => void renamePreset(file, preset.id, name));
}

/** deleted = a tombstone: it leaves the list, every piece that names it keeps drawing it (§12.2). Undo: back in the list */
async function remove(presets: ContentDoc, preset: { id: string; name: string }): Promise<void> {
	if (!(await confirmDelete(m['creator.preset_delete_confirm']({ name: preset.name })))) return;
	editPresets(presets, (file) => void removePreset(file, preset.id));
	toast(m['creator.preset_deleted']({ name: preset.name }), undo(() => editPresets(presets, (file) => void restorePreset(file, preset.id))));
}
