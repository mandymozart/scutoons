// The texts of the media tooling, the same in every tool (de/en, like @scutoons/auth's labels). The app passes its
// locale once (setupMedia); `labels` there overrides single texts.
import type { AssetKind, AssetQuality } from '@scutoons/api';

export interface MediaLabels {
	/** the asset picker */
	pickerTitle: string;
	close: string;
	resize: string;
	/** the selected asset's × (back to the grid) */
	back: string;
	unavailable: string;
	usedN: (count: number) => string;
	unused: string;
	renditions: string;
	quality: Record<AssetQuality, string>;
	use: string;
	upload: string;
	uploadPath: string;
	replaceTitle: string;
	replaceMessage: (path: string) => string;
	replace: string;
	replaceUnchanged: string;
	search: string;
	folder: string;
	allFolders: string;
	kind: string;
	allKinds: string;
	kinds: Record<AssetKind, string>;
	unusedOnly: string;
	none: string;
	more: string;
	/** the slots */
	choose: string;
	chooseHint: string;
	missing: string;
	remove: string;
	/** the media-variants slot */
	variantsChooseHint: string;
	files: string;
	filesHint: string;
	levelReplace: string;
	levelMissing: string;
	levelClear: (level: string) => string;
	defaultLevel: string;
	single: string;
}

export const MEDIA_LABELS: Record<'de' | 'en', MediaLabels> = {
	de: {
		pickerTitle: 'Datei wählen',
		close: 'Schließen',
		resize: 'Ziehen, um das Panel breiter oder schmaler zu machen',
		back: 'Zurück zur Liste',
		unavailable: 'Die Asset-Verwaltung ist auf diesem Server noch nicht eingerichtet.',
		usedN: (count) => `${count}× benutzt`,
		unused: 'unbenutzt',
		renditions: 'Dateien',
		quality: { low: 'Niedrig', medium: 'Mittel', high: 'Hoch' },
		use: 'Verwenden',
		upload: 'Hochladen',
		uploadPath: 'Pfad im CDN',
		replaceTitle: 'Datei ersetzen?',
		replaceMessage: (path) =>
			`${path} gibt es schon. Die neue Datei ersetzt sie überall, wo sie verwendet wird; die alte bleibt als frühere Fassung erhalten.`,
		replace: 'Ersetzen',
		replaceUnchanged: 'Die Datei ist identisch mit der aktuellen – nichts zu ersetzen.',
		search: 'Name, Pfad, Tag suchen',
		folder: 'Ordner',
		allFolders: 'Alle Ordner',
		kind: 'Art',
		allKinds: 'Alle Arten',
		kinds: { image: 'Bilder', video: 'Videos', audio: 'Audio', archive: 'Archive (zip)', document: 'Dokumente', other: 'Andere' },
		unusedOnly: 'Nur unbenutzte',
		none: 'Nichts gefunden.',
		more: 'Mehr',
		choose: 'Auswählen …',
		chooseHint: 'Eine Datei aus der Asset-Verwaltung wählen oder hochladen',
		missing: 'Nicht im CDN',
		remove: 'Entfernen',
		variantsChooseHint: 'Asset wählen: alle Qualitätsdateien (480p / 720p / 1080p) auf einmal, 720p als Standard',
		files: 'Dateien je Qualität',
		filesHint:
			'Eine Datei je Qualitätsstufe (der Leser wählt sie im Konto oder „Automatisch“ nach seiner Verbindung). Fehlt eine Stufe, nimmt der Reader die nächstniedrigere, dann den Standard.',
		levelReplace: 'Diese Datei ersetzen (die Auswahl zeigt die Dateien ihres Assets)',
		levelMissing: 'Nicht gesetzt – eine Datei für diese Qualität wählen',
		levelClear: (level) => `${level} entfernen`,
		defaultLevel: 'Standard',
		single: 'Eine Datei für jede Qualität'
	},
	en: {
		pickerTitle: 'Choose a file',
		close: 'Close',
		resize: 'Drag to resize the panel',
		back: 'Back to the list',
		unavailable: 'The asset manager is not set up on this server yet.',
		usedN: (count) => `used ${count}×`,
		unused: 'unused',
		renditions: 'Files',
		quality: { low: 'Low', medium: 'Medium', high: 'High' },
		use: 'Use',
		upload: 'Upload',
		uploadPath: 'Path on the CDN',
		replaceTitle: 'Replace the file?',
		replaceMessage: (path) =>
			`${path} is already there. The new file takes its place everywhere it is used; the old one is kept as a previous version.`,
		replace: 'Replace',
		replaceUnchanged: 'The file is identical to the current one – nothing to replace.',
		search: 'Search name, path, tag',
		folder: 'Folder',
		allFolders: 'All folders',
		kind: 'Kind',
		allKinds: 'All kinds',
		kinds: { image: 'Images', video: 'Videos', audio: 'Audio', archive: 'Archives (zip)', document: 'Documents', other: 'Other' },
		unusedOnly: 'Unused only',
		none: 'Nothing found.',
		more: 'More',
		choose: 'Choose …',
		chooseHint: 'Pick a file from the asset manager or upload one',
		missing: 'Not found on the CDN',
		remove: 'Remove',
		variantsChooseHint: 'Pick an asset: all its quality files (480p / 720p / 1080p) at once, 720p the default',
		files: 'Files per quality',
		filesHint:
			'One file per quality level (readers choose it in their account, or "Automatic" by their connection). A missing level falls back to the next lower one, then to the default.',
		levelReplace: 'Replace this file (the picker lists the files of its asset)',
		levelMissing: 'Not set – choose a file for this quality',
		levelClear: (level) => `Remove ${level}`,
		defaultLevel: 'Default',
		single: 'One file for every quality'
	}
};

/** the texts for a locale (anything but `de` → English), single texts overridden */
export function mediaLabels(locale: string, overrides: Partial<MediaLabels> = {}): MediaLabels {
	return { ...MEDIA_LABELS[locale === 'de' ? 'de' : 'en'], ...overrides };
}
