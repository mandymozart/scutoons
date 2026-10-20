// The storyboard import (#120, agents/research/STUDIO-IN-CREATOR.md §2.2): the scene files
// storyboards/<toon>/s<season>c<chapter>/scene-<n>.json → one SQL INSERT per chapter that stores
// `storyboard:<toon>:<chapter>` = `{ storyboard: 2, toon, chapter, scenes }` (scenes sorted by source.scene, each file's
// content as it is) as a published version – only when the entity doesn't exist yet and the toon does (idempotent; a
// database without the toon – the test database – stays empty). The CLI (main.ts) writes the migration file.

export interface SceneFile {
	/** the file's path, for messages */
	path: string;
	content: unknown;
}

export interface ChapterStoryboard {
	storyboard: 2;
	toon: string;
	chapter: number;
	scenes: Record<string, unknown>[];
}

const TOON = /^[a-z0-9_-]{1,64}$/;

/** `s1c3` → 3 (the chapter folder of a season), null when the name is none */
export function chapterOfFolder(name: string): number | null {
	const m = /^s[1-9]\d*c(0|[1-9]\d*)$/.exec(name);
	return m ? Number(m[1]) : null;
}

/** One chapter's document from its scene files; throws on a file of another toon / chapter or a doubled scene */
export function chapterDoc(toon: string, chapter: number, files: SceneFile[]): ChapterStoryboard {
	if (!TOON.test(toon)) throw new Error(`invalid toon ${toon}`);
	const scenes = files.map(({ path, content }) => {
		if (!content || typeof content !== 'object' || Array.isArray(content)) throw new Error(`${path}: not a JSON object`);
		const scene = content as Record<string, unknown>;
		if (scene.storyboard !== 2) throw new Error(`${path}: not a storyboard 2 scene file`);
		if (scene.toon !== toon) throw new Error(`${path}: toon ${String(scene.toon)}, expected ${toon}`);
		if (scene.chapter !== undefined && scene.chapter !== chapter) throw new Error(`${path}: chapter ${String(scene.chapter)}, expected ${chapter}`);
		const number = (scene.source as { scene?: unknown } | undefined)?.scene;
		if (typeof number !== 'number') throw new Error(`${path}: no source.scene`);
		return { number, scene };
	});
	scenes.sort((a, b) => a.number - b.number);
	for (let i = 1; i < scenes.length; i++) {
		if (scenes[i].number === scenes[i - 1].number) throw new Error(`scene ${scenes[i].number} twice in ${toon} chapter ${chapter}`);
	}
	return { storyboard: 2, toon, chapter, scenes: scenes.map((s) => s.scene) };
}

/** A MySQL string literal (backslash escapes, as the server's ImportContentCommand::sqlString – one line, no raw newline) */
export function sqlString(value: string): string {
	const map: Record<string, string> = { '\\': '\\\\', "'": "\\'", '\0': '\\0', '\n': '\\n', '\r': '\\r', '\x1a': '\\Z' };
	return `'${value.replace(/[\\'\0\n\r\x1a]/g, (c) => map[c])}'`;
}

/** The guarded INSERT of one chapter's document (published, author NULL – an import) */
export function insertStatement(doc: ChapterStoryboard, migration: string, createdAt: string): string {
	const key = `storyboard:${doc.toon}:${doc.chapter}`;
	const json = JSON.stringify(doc);
	return (
		'INSERT INTO `entity_versions` (`entity_key`, `kind`, `toon`, `locale`, `json`, `author`, `created_at`, `status`, `message`)\n' +
		`SELECT ${sqlString(key)}, 'storyboard', ${sqlString(doc.toon)}, NULL, ${sqlString(json)}, NULL, ${sqlString(createdAt)}, 'published', ${sqlString(`import (${migration})`)} FROM DUAL\n` +
		`WHERE NOT EXISTS (SELECT 1 FROM \`entity_versions\` v WHERE v.\`entity_key\` = ${sqlString(key)})\n` +
		`  AND EXISTS (SELECT 1 FROM \`entity_versions\` t WHERE t.\`entity_key\` = ${sqlString(`toon:${doc.toon}`)});\n`
	);
}

/** The whole migration file: a header and one statement per chapter, in chapter order */
export function migrationSql(docs: ChapterStoryboard[], migration: string, createdAt: string, header: string): string {
	const sorted = [...docs].sort((a, b) => a.toon.localeCompare(b.toon) || a.chapter - b.chapter);
	return header + sorted.map((d) => insertStatement(d, migration, createdAt)).join('');
}
