// A shot's generated files and their records (agents/research/VARIANTS-CANVAS.md §3.7) – the routes api.mjs spreads in:
//   GET  /api/generated?ch&ids=a,b     – the picked shots' records, shot by shot in that order, each in log order:
//        [{ id, shot, file, url, kind, take, parents, inputs, generator, model, mode, job, status, prompt, seed, ratio,
//           duration, ms (a video file's real length), estimate, cost, date, error?, params?, approved, mark, marked,
//           before (the mark the last mark line replaced – what "X again" brings back), missing (a record whose file is
//           gone), earlier (a file without a v2 record – the board's row "earlier – no record"; never guessed) }]
//   POST /api/generated { ch, id, name, data | url } – a dropped file (png / jpeg / webp, mp4 / webm) → <shot>--g<n>.<ext>
//        (the highest n + 1) + its record { generator: 'upload' }
//   POST /api/generated/approve { toon, ch, id, file, note } – approve.mjs (review + dataset, `from`), then the
//        record's mark back to none (a mark line { mark: null })
//   POST /api/generated/mark { ch, ids, mark: 'review' | 'rejected' | null, marked } – one mark line per id (append-only:
//        no rewrite racing comfy.mjs; the last line per id wins). One X's stamp: the client sends the card and its
//        branch with one `marked`; the chosen can't be rejected.
// The log and its writers: scripts/storyboard/generated-log.mjs. "approved" = approve.mjs's `from` (clips.mjs approvedOf).

import { existsSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { MARKS, appendLine, idOf, kindOf, markLine, readLog, record, shotOf, writeNew } from '../../scripts/storyboard/generated-log.mjs';
import { approvedOf, clipMs } from './clips.mjs';

const CH = /^[a-z0-9-]+$/;
const SHOT = /^[\w-]+$/;
const ID = /^[\w-]+--[\w.-]+$/;
const MEDIA = /\.(png|jpe?g|webp|mp4|webm|mov|m4v)$/i;
const TYPES = { 'image/png': 'png', 'image/jpeg': 'jpg', 'image/webp': 'webp', 'video/mp4': 'mp4', 'video/webm': 'webm' };
const nOf = (f) => Number(/--g(\d+)\./.exec(f)?.[1] ?? 0);
const checkCh = (ch) => { if (!CH.test(ch ?? '')) throw new Error('bad chapter'); };

export function generatedRoutes({ out, boards, run, now = () => new Date(), fetchImpl = fetch }) {
	const dirOf = (ch) => resolve(out, ch, 'generated');
	const filesOf = (ch) => (existsSync(dirOf(ch)) ? readdirSync(dirOf(ch)) : []);

	/** the picked shots' records (the shape above) */
	function records(ch, ids) {
		checkCh(ch);
		const shots = String(ids).split(',').filter(Boolean);
		if (shots.some((s) => !SHOT.test(s))) throw new Error('bad id');
		const log = readLog(out, ch);
		const all = filesOf(ch).filter((f) => MEDIA.test(f));
		const list = [];
		for (const shot of shots) {
			const files = all.filter((f) => f.startsWith(`${shot}--`));
			const approved = approvedOf({ boards, ch, shot, files });
			const extra = (file) => ({
				url: file ? `/out/${ch}/generated/${encodeURIComponent(file)}` : null,
				ms: file && kindOf(file) === 'video' ? clipMs(resolve(dirOf(ch), file)) : null,
				approved: !!file && approved.has(file)
			});
			const seen = new Set();
			for (const { v, ...r } of log.records.values()) {
				if (r.shot !== shot) continue;
				if (r.file) seen.add(r.file);
				list.push({ ...r, ...extra(r.file), missing: !!r.file && !files.includes(r.file), earlier: false });
			}
			// files without a v2 record (an old line still in the log, or none): only what the file itself says
			for (const file of files.filter((f) => !seen.has(f)).sort((a, b) => nOf(a) - nOf(b) || a.localeCompare(b))) {
				const id = idOf(file);
				list.push({
					id, shot, file, kind: kindOf(file), take: null, generator: null, model: null, mode: null, job: null, status: null,
					parents: [], inputs: [], prompt: null, seed: null, ratio: null, duration: null, estimate: null, cost: null, date: null,
					mark: null, marked: null, before: null, ...log.marks.get(id), ...extra(file), missing: false, earlier: true
				});
			}
		}
		return list;
	}

	async function drop({ ch, id, name, data, url }) {
		checkCh(ch);
		if (!SHOT.test(id ?? '')) throw new Error('bad id');
		if (!data && url) {
			if (!/^https?:\/\//.test(url)) throw new Error('not an image link');
			const r = await fetchImpl(url);
			const type = (r.headers.get('content-type') ?? '').split(';')[0];
			if (!r.ok || !TYPES[type]) throw new Error(`the link gave ${r.ok ? type || 'no image' : r.status} – save the file and drop it`);
			data = `data:${type};base64,${Buffer.from(await r.arrayBuffer()).toString('base64')}`;
			name ??= url.split('/').pop().split('?')[0];
		}
		const m = /^data:([\w/+.-]+);base64,/.exec(data ?? '');
		if (!m || !TYPES[m[1]]) throw new Error('png / jpeg / webp / mp4 / webm only');
		const { id: gid, file } = writeNew(out, ch, id, TYPES[m[1]], Buffer.from(data.slice(m[0].length), 'base64'));
		appendLine(out, ch, record({ id: gid, shot: id, file, generator: 'upload', date: now().toISOString(), params: { name: name ?? null, url: url ?? null } }));
		return { file, id: gid, from: name ?? '' };
	}

	async function approve({ toon, ch, id, file, note }) {
		checkCh(ch);
		if (!CH.test(toon ?? '') || !SHOT.test(id ?? '')) throw new Error('bad id');
		if (!/^[\w.-]+$/.test(file ?? '') || !file.startsWith(`${id}--`)) throw new Error('bad file');
		const r = await run('scripts/storyboard/approve.mjs', [`storyboards/${toon}/${ch}`, id, '--image', resolve(dirOf(ch), file), ...(note ? ['--note', note] : [])]);
		// chosen is no mark: approving takes the card's review / rejected back to none
		if (r.ok) appendLine(out, ch, markLine(idOf(file), null, null));
		return r;
	}

	function mark({ ch, ids, mark = null, marked }) {
		checkCh(ch);
		if (!Array.isArray(ids) || !ids.length || ids.some((x) => !ID.test(String(x)))) throw new Error('bad ids');
		if (mark !== null && !MARKS.includes(mark)) throw new Error(`bad mark ${mark} (review · rejected · null)`);
		const stamp = marked ?? (mark ? now().toISOString() : null);
		if (stamp !== null && (typeof stamp !== 'string' || stamp.length > 40 || Number.isNaN(Date.parse(stamp)))) throw new Error('bad marked');
		const log = readLog(out, ch);
		const known = new Set(filesOf(ch).map(idOf));
		const unknown = ids.filter((x) => !log.records.has(x) && !known.has(x));
		if (unknown.length) throw new Error(`no generation ${unknown.join(', ')}`);
		if (mark === 'rejected') {
			const files = filesOf(ch).filter((f) => MEDIA.test(f));
			const chosen = [...new Set(ids.map(shotOf))].flatMap((shot) => [...approvedOf({ boards, ch, shot, files: files.filter((f) => f.startsWith(`${shot}--`)) })].map(idOf));
			const hit = ids.filter((x) => chosen.includes(x));
			if (hit.length) throw new Error(`${hit.join(', ')} is chosen – approve another first`);
		}
		for (const x of ids) appendLine(out, ch, markLine(x, mark, stamp));
		return { ids, mark, marked: stamp };
	}

	return {
		// the board's one request (the Variants panel's `?ch&id` – a shot's file names – left with it)
		'GET /api/generated': ({ ch, ids }) => {
			if (ids == null) throw new Error('ids missing – GET /api/generated?ch=<ch>&ids=<shot>,<shot>');
			return records(ch, ids);
		},
		'POST /api/generated': drop,
		'POST /api/generated/approve': approve,
		'POST /api/generated/mark': mark
	};
}
