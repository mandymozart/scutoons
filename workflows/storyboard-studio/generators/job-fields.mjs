// A job's inputs as fields and as the record keeps them (agents/research/VARIANTS-CANVAS.md §3.6, §3.7) – pure, no
// imports: the server (jobs.mjs, index.mjs, generated.mjs) and the studio's page (Improve on · Vary · More) use the same
// rules. README "The record".
//   A job (POST /api/generate's `job`, the manual slot's `job`) names its inputs as keys (generators/index.mjs: "out:…",
//   "boards:…", "media:…") in its fields: still · control · characters[] · style[]. A still can be a FRAME of a video
//   under _out: "frame:<video key>#first|last|<ms>" (index.mjs cuts it – frames.mjs).
//   The record keeps them as [{ role, key, frame?, at? }], the main input first (inputsOf) – a frame as its VIDEO's key
//   + `frame` (its parent is the video); fieldsOf turns a record's inputs back into the fields (Vary: the card's own).

export const FRAME_MARKS = ['first', 'last'];
const FRAME = /^frame:(out:[^#]+)#(first|last|\d{1,9})$/;

/** "frame:out:c1/generated/s--g4.mp4#first" → { key: 'out:c1/generated/s--g4.mp4', frame: 'first' } (ms → a number);
 *  anything else → null */
export function parseFrameKey(key) {
	const m = FRAME.exec(String(key ?? ''));
	return m ? { key: m[1], frame: /^\d+$/.test(m[2]) ? Number(m[2]) : m[2] } : null;
}

/** a video's key + 'first' | 'last' | ms → its frame's key ("frame:<key>#<frame>"; ms rounded, ≥ 0) */
export function frameKey(key, frame) {
	const ms = typeof frame === 'number' ? frame : /^\d+(?:\.\d+)?$/.test(String(frame)) ? Number(frame) : NaN;
	const f = FRAME_MARKS.includes(frame) ? frame : ms >= 0 && ms < 1e9 ? Math.round(ms) : null;
	if (f === null || !/^out:[^#]+$/.test(String(key ?? ''))) throw new Error(`bad frame ${key}#${frame} – a video under out: and first | last | ms`);
	return `frame:${key}#${f}`;
}

/** a field's key as a record's input: a frame → its video's key + `frame` */
function input(role, key, extra = {}) {
	const f = parseFrameKey(key);
	return f ? { role, key: f.key, frame: f.frame, ...extra } : { role, key: String(key), ...extra };
}

/** the role a model's still plays: the keyframe at 0 of a video to video, the picture attached by hand (Luma's image
 *  reference) for a manual one, else the still (an image to video's first frame, comfy-local's image 1 – the edit) */
export const stillRole = (m) => (m?.mode === 'video_to_video' ? 'keyframe' : m?.mode === 'manual' ? 'reference' : 'still');

/** what a job sends, as keys with their roles – only what the model takes, the main input first: a video to video's
 *  control video (the still is its keyframe at 0), else the still */
export function inputsOf(m, job) {
	const takes = (what) => (m?.inputs ?? []).includes(what);
	const list = [];
	if (takes('control') && job?.control) list.push({ role: 'control', key: String(job.control) });
	if (takes('still') && job?.still) list.push(stillRole(m) === 'keyframe' ? input('keyframe', job.still, { at: 0 }) : input(stillRole(m), job.still));
	if (takes('characters')) for (const k of job?.characters ?? []) list.push({ role: 'character', key: String(k) });
	if (takes('style')) for (const k of job?.style ?? []) list.push({ role: 'style', key: String(k) });
	return list;
}

/** a record's inputs → a job's fields (Vary: the card's own inputs): { still, control, characters, style, missing } –
 *  the first still / keyframe / reference is the still (a frame as its "frame:" key), the first control the control;
 *  `missing` = the inputs the record names without a key (the conversion couldn't resolve them) */
export function fieldsOf(inputs) {
	const f = { still: null, control: null, characters: [], style: [], missing: [] };
	for (const i of inputs ?? []) {
		if (!i?.key) { f.missing.push(i?.name ?? i?.role ?? '?'); continue; }
		const key = i.frame != null ? frameKey(i.key, i.frame) : i.key;
		if (i.role === 'control') f.control ??= key;
		else if (i.role === 'still' || i.role === 'keyframe' || i.role === 'reference') f.still ??= key;
		else if (i.role === 'character') f.characters.push(key);
		else if (i.role === 'style') f.style.push(key);
	}
	return f;
}
