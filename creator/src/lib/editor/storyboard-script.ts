// The script beside the storyboard (#123): a scene's lines out of the toon's screenplay (`script:<toon>:<locale>`, the
// source language – as the toon's Script page loads it), found by the screenplay's own outline rules (@scutoons/content
// scriptOutline: a scene counts inside a chapter). Replaces the studio's `docker exec` reading (STUDIO-IN-CREATOR §1.2).
import { scriptOutline } from '@scutoons/content';

/** a scene's lines: from its heading to the line before the next chapter / scene / song marker; null = not in the script */
export function sceneLines(text: string, scene: number): string[] | null {
	const outline = scriptOutline(text);
	const at = outline.findIndex((o) => o.kind === 'scene' && o.number === scene);
	if (at < 0) return null;
	const lines = text.split('\n');
	const from = outline[at].line - 1;
	const to = at + 1 < outline.length ? outline[at + 1].line - 1 : lines.length;
	const out = lines.slice(from, to);
	// the blank lines at its end belong to the gap, not the scene
	while (out.length > 1 && !out[out.length - 1].trim()) out.pop();
	return out;
}
