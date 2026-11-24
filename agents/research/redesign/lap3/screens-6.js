/* lap 3 draft – the owner's decisions of 2026-11-24 laid over the screens (NOTES.md note 34, lap3/DECISIONS.md): an answered "Owner decides"
   card becomes a green "Decided" note in its place; cards not answered stay. Loaded last. */
document.head.insertAdjacentHTML('beforeend', '<style>:root,[data-theme]{--tool-panel:color-mix(in srgb,var(--tool-ground) 92%,transparent)} .q.done{background:#e4f3dd;border:2px solid #4c8f39;color:#1d2a16} .q.done b{color:#2f6a1f}</style>');
const Q_RE = /<div class="q" style="([^"]*)"><b>Owner decides:[\s\S]*?<\/ol>(?:<div class="why">[\s\S]*?<\/div>)?<\/div>/g;
/* notes: one per card on the screen, in order; undefined leaves that card as it is */
const decide = (id, notes, extra = '') => {
  const s = S.find(x => x.id === id); if (!s) return;
  const r = s.render;
  s.render = () => { let k = 0; return r().replace(Q_RE, (m, style) => { const t = notes[k++]; return t === undefined ? m : `<div class="q done" style="${style}"><b>Decided 2026-11-24</b>${t}</div>`; }) + extra; };
};

decide('M1', ['Both apps get both: the … on the hovered or picked row, and the right click – the same menu either way.']);
decide('M2', ['A level below opens after a short rest of about 150 ms.']);
decide('M3', ['The menu that read as tree-like is the Library\'s places list (This item · This toon · By cast → the seasons) – taken from your "1"; say so if you meant another. Its redraw follows the scope question (P2b).']);
decide('T1', ['A footer in every tree – Add · copy · group · delete, always in the same place; the same four in the row\'s … menu and on the keys.']);
decide('T2', ['One bar on the stage, top centre – the same place in every workspace.']);
decide('L1', ['Upload is a quiet icon at the end of the Find field. The dark glow on filled buttons at night is a fault in code – a fix, in the notes for the plan.']);
decide('L2', ['A picked entry\'s details show in the Inspector; a wide Library keeps a column of its own. Nothing opens by itself.']);
decide('L3', ['Yes – every slot in both apps takes its content this one way.']);
decide('V1', ['Top left of its cell, as a tab word.']);
decide('V2', ['The panel\'s ground: 92 % – every lap 3 screen draws panels so now.', 'A menu\'s ground: its own value, 94 %.', 'A spore\'s body is the panel\'s ground under its words; the canvas dots step back to half.']);
decide('S1', ['The unit is a "section". Chapter and Storyboard become one workspace: the same strip – the lens set to Placed is the chapter. (Which section is in which state is made up for the picture.)'],
  qcard(690, 580, 'what the one workspace is called', ['Chapter – it is the thing being made; "Storyboard" lives on as the Blocked lens (recommended)', 'Storyboard', 'Story'], 'The bar on these pictures still shows both names until you say.', '29.5rem'));
decide('S2', ['The engine\'s own view inside the creator through wasm, as the Spores canvas already is – one code for both apps. (Figures, poses and lens names are made up for the picture.)']);
decide('S3', ['One timeline component of the kit for all four uses, in both apps – built once, starting with the Storyboard\'s.']);
decide('S4', ['Yes – one thing that plays (a clip), three ways to say when: by an event (spores), by being launched (a grid), by its place in time (a timeline).']);
decide('P1', ['One view: Flow is the story level of Spores (universe → toon → season → chapter → its events); the Flow workspace becomes a preset of it. Flow\'s own renderer and the Generate stub go.']);
decide('P3', ['The hardware layer lives with the deck and the venue (the stage environment): the show says "Brightness", this deck says which knob or lamp.', 'Only in the deck – taken from your "1"; the creator shows the finished knobs. The creator\'s Parameters and Assignment tabs go with the rest of Sound design.']);
