// Headless: one storyboard shot → the page shows exactly its image (canvas 1:1), render-shots.mjs screenshots it.
// render.html?spec=<file url of the shots module>&id=<shot id>&three=<three's folder>. The work is in core.js.
// An error is written onto the page (it lands in the PNG), so a broken frame says why.
import { loadFonts, renderShot } from './core.js';

try {
	const fonts = loadFonts(); // in parallel with the scene
	const params = new URLSearchParams(location.search);
	const { default: SHOTS } = await import(params.get('spec'));
	const shot = SHOTS.find((s) => s.id === params.get('id')) ?? SHOTS[0];
	await fonts;
	const { canvas, clean } = await renderShot(shot, { clean: true });
	for (const c of [canvas, clean]) {
		c.style.cssText = `display:block;width:${shot.w}px;height:${shot.h}px`;
		document.body.appendChild(c); // the frame on top, the clean image (no texts, no marks – for generators) below
	}
} catch (e) {
	document.body.innerHTML = `<pre style="color:#f55;font:24px monospace;white-space:pre-wrap;padding:20px">${String(e?.stack ?? e)}</pre>`;
}
document.title = 'done';
