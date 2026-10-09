// Headless: one storyboard shot → the page shows exactly its image (canvas 1:1), render-shots.mjs screenshots it.
// render.html?spec=<file url of the shots module>&id=<shot id>&three=<three's folder>. The work is in core.js.
import { renderShot } from './core.js';

const params = new URLSearchParams(location.search);
const { default: SHOTS } = await import(params.get('spec'));
const shot = SHOTS.find((s) => s.id === params.get('id')) ?? SHOTS[0];
const { canvas } = await renderShot(shot);
canvas.style.cssText = `display:block;width:${shot.w}px;height:${shot.h}px`;
document.body.appendChild(canvas);
document.title = 'done';
