"""Build live text overlays (``Slice.dialog``) for chapter 1 "Leugnen" from hand-transcribed boxes.

The German text of chapter 1 is baked into ``images/3x/de/S1C1 - Slice N.jpg``. ``extract_bubbles.py``
finds candidate boxes; the transcription, translation and a seed bbox per box live in ``BOXES`` below.
This script measures each caption box in the image (outer border, text lines, glyph advance -> font size)
and writes

- ``viewer/static/api/catpope/i18n/s1c1.json``              ``{id: {de, en, slice}}``
- the ``dialog`` key of the slices in ``viewer/static/api/catpope/{de,en}/chapter1.json``
  (text-level edit: only the ``dialog`` key is added/replaced, everything else stays byte-identical)

Rendering strategy (see agents/DESIGN.md Part 2 + viewer toon/dialog/dialog.svelte):

- ``kind="box"``: a complete bordered caption box. Rendered as variant ``caption`` (upright) or ``narration``
  (italic) with ``rect`` = the box's outer border, so the renderer's paper box + 2px ink border redraws the
  baked box exactly and covers the German text.
- boxes crossing a slice seam are hosted by the *later* slice with a negative ``rect.y`` (later slices paint
  over earlier ones, so the overlay covers both halves of the baked box).
- ``kind="text"``: text that can't be redrawn as a box (box cut by the slice edge, dashed border, SFX or notes
  overlapping the box, end card). Rendered as variant ``bubble`` with ``font: mono`` and a mask that covers only
  the text area.

Usage:  cdn/.venv/Scripts/python cdn/tools/s1c1_dialogs.py [--dry-run]
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys

import numpy as np
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
IMG = os.path.join(ROOT, "cdn", "root", "catpope", "images", "3x", "de", "S1C1 - Slice {}.jpg")
I18N = os.path.join(ROOT, "viewer", "static", "api", "catpope", "i18n", "s1c1.json")
CHAPTERS = {
    "de": os.path.join(ROOT, "viewer", "static", "api", "catpope", "de", "chapter1.json"),
    "en": os.path.join(ROOT, "viewer", "static", "api", "catpope", "en", "chapter1.json"),
}
W, H = 1170, 1875
LINE_HEIGHT = 1.35   # renderer: .font-mono line-height
ADVANCE = 0.6        # IBM Plex Mono advance width in em

# slice: slice whose image holds the (start of the) text; seed: rough text bbox [x0,y0,x1,y1] in that slice
# style: 'upright' -> caption, 'italic' -> narration. split: box continues into slice+1 (hosted there).
# kind: 'box' (redraw caption box) | 'text' (mask text only). Optional: align, valign, uppercase, rotation,
# right (max right edge of the text mask, px), rect (explicit text rect px [x0,y0,x1,y1] for kind=text).
BOXES = [
    dict(slice=2, seed=[140, 215, 917, 250], style="italic", uppercase=True,
         de="2 INT. SCHLAFZIMMER - NACHT",
         en="2 INT. BEDROOM - NIGHT"),
    dict(slice=2, seed=[107, 617, 556, 714], style="italic", uppercase=True,
         de="EIN NEUER TAG,\nEIN ALTES SPIEL!",
         en="A NEW DAY,\nTHE SAME OLD GAME!"),
    dict(slice=2, seed=[609, 1250, 889, 1284], style="upright", uppercase=True,
         de="MAMA! WTF?",
         en="MOM! WTF?"),
    dict(slice=3, seed=[226, 993, 959, 1030], style="upright",
         de="Puh! Es war nur ein Traum.",
         en="Phew! It was only a dream."),
    dict(slice=3, seed=[50, 1314, 695, 1360], style="italic",
         de="PLF schlief wieder ein.",
         en="PLF fell back asleep."),
    dict(slice=4, seed=[140, 1774, 1002, 1867], style="italic", split=True, uppercase=True,
         de="3 EXT. BRUTALISTISCHER WOHNBLOCK\n- SONNENAUFGANG",
         en="3 EXT. BRUTALIST HOUSING BLOCK\n- SUNRISE"),
    dict(slice=5, seed=[138, 1205, 1028, 1920], style="italic", split=True,
         de="An jedem Morgen ging die Sonne\nauf, so war es geschrieben. Es\nverschob sich, das wussten wir,\n"
            "je nach Stellung der Erde zum\nGestirn, und wenn es regnete oder\nzu viel Nebel gab, dann konnten\n"
            "wir dennoch sicher sein, dass es\ndahinter, seit Beginn der Zeit,\ndasselbe kosmische Ritual war.\n"
            "Auf Schatten folgte Licht, und\ndas Licht war das Leben, und mit\ndem Leben begann auch mein Tag.",
         en="Every morning the sun rose, so it was written. It shifted, we knew that, with the position of the "
            "Earth relative to its star, and when it rained or there was too much fog, we could still be sure "
            "that behind it all, since the beginning of time, the same cosmic ritual was taking place. Shadow "
            "was followed by light, and the light was life, and with life my day began too."),
    dict(slice=7, seed=[123, 782, 929, 1065], style="italic",
         de="Um dennoch sicherzugehen, dass\ndieser Tag auch funktionieren\nkonnte, stellte ich mir einen\n"
            "Wecker, einen digitalen mit\nAtomuhrverbindung.",
         en="Still, to make sure this day would actually work, I set myself an alarm clock, a digital one, "
            "synced to an atomic clock."),
    dict(slice=7, seed=[124, 1336, 850, 1369], style="italic", uppercase=True,
         de="4 BEZAHLWERBUNG - SIN TONIC",
         en="4 PAID AD - SIN TONIC"),
    dict(slice=9, seed=[143, 607, 701, 700], style="italic", uppercase=True,
         de="5 INT. SCHLAFZIMMER -\nSONNENAUFGANG",
         en="5 INT. BEDROOM -\nSUNRISE"),
    dict(slice=9, seed=[107, 1052, 992, 1386], style="italic",
         de="Wir predigenden Wesen hatten die\nPflicht, als erste vor den\nGemein(d)en zu erscheinen und die\n"
            "eigenen Rituale vor diesen zu\nerledigen. Daß gehörte eben\ndazu...",
         en="We preaching beings had a duty to be the first to appear before the flock/folk and to perform "
            "our own rituals in front of them. That was just part of it..."),
    dict(slice=10, seed=[320, 1457, 981, 1492], style="italic",
         de="Es machte mir nichts aus.",
         en="I didn't mind."),
    dict(slice=11, seed=[113, 1139, 972, 1962], style="italic", split=True,
         de="Dennoch, ich war geübt darin,\ndieses Ringen gekonnt zu\nignorieren. Ja, geübt in\n"
            "der Technik des Lucid Dreamings,\neiner Methode, in der der\nSchlummerzustand genutzt werden\n"
            "konnte, um die eigenen Träume zu\nsteuern. Eine wahnsinnig\nspannende Methode; es war, als\n"
            "erlebte und schrieb man\ngleichzeitig einen Film in\naller Realität. Ein\nunbeschreibliches\n"
            "Gefühl der Macht.",
         en="Still, I was well practiced at ignoring this ringing. Yes, practiced in the technique of lucid "
            "dreaming, a method in which the state of slumber could be used to steer your own dreams. An "
            "insanely exciting method; it was like living and writing a film at the same time, in full "
            "reality. An indescribable feeling of power."),
    dict(slice=12, seed=[140, 1757, 917, 1980], style="italic", split=True,
         de="Ich zwang das Klingeln in ein\nTrillern. Es vertausendfachte\nsich wie bei einem\nDMT Trip.",
         en="I forced the ringing into a trill. It multiplied a thousandfold, like on a DMT trip."),
    dict(slice=14, seed=[99, 1421, 1040, 1944], style="italic", split=True,
         de="Ich verdichte diese weiter, bis sie\nheulenden Siren wurden.\nDie Siren nahm ich wiederum als\n"
            "Sample. Stabilisierte sie.\nZeichnete mit ihnen eine\nunsichtbare Schicht Engelhorn-\n"
            "Trompeten in den Raum und brachte\ndie Wolken in der obersten\nHemisphäre zum Schwingen.",
         en="I compressed it further until it became howling sirens.\nI took the sirens in turn as a sample. "
            "Stabilized them. With them I drew an invisible layer of angel-horn trumpets into the room and set "
            "the clouds in the highest hemisphere vibrating."),
    dict(slice=17, seed=[95, 1710, 541, 1744], style="upright",
         de="Zuerst zur Küche.",
         en="Kitchen first."),
    dict(slice=18, seed=[295, 1280, 470, 1314], style="upright",
         de="Eiweis!",
         en="Protein!"),
    dict(slice=18, seed=[141, 1538, 458, 1579], style="upright", uppercase=True,
         de="6 INT. KÜCHE",
         en="6 INT. KITCHEN"),
    dict(slice=19, seed=[142, 962, 910, 1065], style="upright",
         de="PLF schlug gekonnt zwei Eier\nsynchron in einer Pfanne auf.",
         en="PLF expertly cracked two eggs into a pan, perfectly in sync."),
    dict(slice=20, seed=[131, 770, 1016, 1045], style="upright",
         de="Plötzliche Drehung. In der Luft\nwaren Bremsspuren. Mit\nhalb geschlossenen Augen ging PLF\n"
            "ins Klo. Nahm ein Buch.\nSetzte sich.",
         en="A sudden turn. Skid marks in the air. Eyes half closed, PLF went to the loo. Took a book. "
            "Sat down."),
    dict(slice=21, seed=[141, 661, 405, 694], style="upright", uppercase=True,
         de="7 INT. KLO",
         en="7 INT. LOO"),
    dict(slice=22, seed=[458, 206, 1012, 361], style="upright",
         de="Nunja, bissi profan,\ntaugt aber zum\nGeschichten erzählen.",
         en="Well, a bit profane, but good for telling stories."),
    dict(slice=22, seed=[124, 1403, 657, 1566], style="upright",
         de="Er machte drei Ringe\nund gab einen an\njeden Sohn?",
         en="He made three rings and gave one to each son?"),
    dict(slice=23, seed=[654, 1689, 959, 1726], style="italic",
         de="Dachte ich!",
         en="I knew it!"),
    dict(slice=24, seed=[101, 1187, 1015, 1710], style="upright",
         de="Wer bestimmt die Wahrheit?\nWer hält die Schlüssel zu den\nGeschichten?\n"
            "Wo liegt der Unterschied zwischen\nmeinem Wollen und dem, was ich für\nandere inszeniere?\n"
            "Wem sollte ich dienen, wenn nicht\ndrei Herren, der perfekten\nDreieinigkeit des Chaos?",
         en="Who decides what is true?\nWho holds the keys to the stories?\nWhere is the difference between "
            "what I want and what I stage for others?\nWhom should I serve, if not three masters, the perfect "
            "trinity of chaos?"),
    dict(slice=25, seed=[101, 546, 815, 1033], style="italic",
         de="Natürlich dauerte die\nLektüre länger als ich es\nwahrhaben wollte. Ich\nneigte dazu, mich im Text\n"
            "zu verlieren. Eingesaugt,\nals wäre ich\nselbst die Magd auf dem\nScheiterhaufen.",
         en="Of course the reading took longer than I cared to admit. I tended to lose myself in the text. "
            "Sucked in, as if I myself were the maid on the pyre."),
    dict(slice=27, seed=[140, 372, 882, 472], style="upright",
         de="Alles Fetisch? Alles Show?\nWill ich nur unterhalten?",
         en="All fetish? All show?\nDo I just want to entertain?"),
    dict(slice=28, seed=[285, 198, 884, 550], style="upright",
         de="Nein, es geht hier um\nmehr als die\nMetapher! Feuer\nentzünden. Sie sollen\n"
            "vor Emotionen mit mir\nbrennen.",
         en="No, this is about more than the metaphor! Light fires. They should burn with emotion, "
            "right along with me."),
    dict(slice=30, seed=[237, 1401, 828, 1440], style="upright", kind="text", size=4.05,
         rect=[229, 1401, 840, 1447],
         de="Holy Moly, Guacamole!",
         en="Holy moly, guacamole!"),
    dict(slice=30, seed=[237, 1462, 520, 1501], style="upright", kind="text", size=4.05,
         rect=[229, 1462, 580, 1512],
         de="Meine Eier!",
         en="My eggs!"),
    dict(slice=31, seed=[141, 677, 458, 717], style="upright", uppercase=True,
         de="8 INT. KÜCHE",
         en="8 INT. KITCHEN"),
    dict(slice=32, seed=[203, 102, 715, 400], style="upright",
         de="Ich ess später ein\npaar Oblaten.\n\nEs gibt\nWichtigeres...",
         en="Communion wafers later.\n\nOther things matter more..."),
    dict(slice=33, seed=[600, 1281, 1038, 1318], style="upright",
         de="... die Routine!",
         en="... the routine!"),
    dict(slice=34, seed=[141, 70, 594, 102], style="upright", uppercase=True,
         de="8 INT. BADEZIMMER",
         en="8 INT. BATHROOM"),
    dict(slice=34, seed=[140, 414, 682, 703], style="upright",
         de="Vorm Waschbecken\nangekommen, ergriff\nPLF die Paste –\ndas blaue Elixier\nextrudierte heraus.",
         en="Arriving at the sink, PLF grabbed the paste – the blue elixir came oozing out."),
    dict(slice=35, seed=[557, 1392, 1070, 1564], style="upright",
         de="Verfehlt? WTF!\nScheint ja gar nix\nmehr zu klappen.",
         en="Missed? WTF!\nNothing seems to work anymore."),
    dict(slice=36, seed=[314, 1398, 1028, 1507], style="italic",
         de="Doch, beim zweite Versuch\ngelang …",
         en="Yes, the second attempt worked …"),
    dict(slice=36, seed=[142, 1651, 300, 1686], style="upright",
         de="Danke!",
         en="Thanks!"),
    dict(slice=37, seed=[113, 1056, 770, 1282], style="upright",
         de="WIR haben es\nverstanden. Ruhe,\njetzt, ich konzentriere\nmich.",
         en="WE got it. Quiet now, I'm concentrating."),
    dict(slice=41, seed=[143, 1197, 482, 1234], style="upright",
         de="Was war das?",
         en="What was that?"),
    dict(slice=42, seed=[570, 475, 625, 495], style="upright", align="center", size=4.05,
         de="...",
         en="..."),
    dict(slice=42, seed=[145, 744, 398, 781], style="upright",
         de="Es war da",
         en="It was there"),
    dict(slice=42, seed=[284, 954, 366, 987], style="upright",
         de="Wer",
         en="Who"),
    dict(slice=42, seed=[485, 1179, 566, 1213], style="upright",
         de="Was",
         en="What"),
    dict(slice=42, seed=[611, 1427, 886, 1516], style="upright",
         de="wessen,\nein Wesen!",
         en="whose,\na being!"),
    dict(slice=43, seed=[145, 84, 568, 184], style="upright",
         de="Ein Katzenvieh?\nIm Bademantel?",
         en="A cat beast?\nIn a bathrobe?"),
    dict(slice=43, seed=[50, 1623, 1080, 1732], style="italic", kind="text",
         de="Nichts mehr! Wie es erschien, war es\nauch wieder weg.",
         en="Nothing! Just as it appeared, it was gone again."),
    dict(slice=44, seed=[346, 1443, 857, 1615], style="upright",
         de="Naja, gut, weg ist\nsie, war wohl nur\nEinbildung.",
         en="Oh well, she's gone, probably just my imagination."),
    dict(slice=46, seed=[141, 1148, 740, 1183], style="upright", uppercase=True,
         de="10 INT. KLEIDERKAMMER",
         en="10 INT. VESTRY"),
    dict(slice=48, seed=[144, 282, 716, 697], style="italic",
         de="Sorgsam und mit\ngeübter Hand glitt\nich in die feine\nGarbadine, deren\n"
            "schlichtes Weiß ihre\nfromme Eleganz\nausstrahlte.",
         en="Carefully, with a practiced hand, I slipped into the fine gabardine, whose plain white radiated "
            "its pious elegance."),
    dict(slice=49, seed=[473, 924, 1006, 1779], style="italic",
         de="Garbadine, mit\nihrer diagonal\ngerippten Struktur,\nlag straff am\nKörper an. Oft zu\n"
            "Unrecht als\ngewöhnlich\nabgetan,\nunterschied sie\nsich wesentlich\nvon der Popeline,\n"
            "die leichter und\nquer gerippt\nwar.",
         en="Gabardine, with its diagonally ribbed weave, lay taut against the body. Often wrongly dismissed "
            "as ordinary, it differed significantly from poplin, which was lighter and ribbed crosswise."),
    dict(slice=51, seed=[119, 729, 717, 1018], style="italic",
         de="Davon konnte ich mich\njedoch nicht weiter\nablenken lassen. Ich\nliebte dieses\nRitual.",
         en="But I couldn't let that distract me any further. I loved this ritual."),
    dict(slice=53, seed=[674, 78, 1013, 184], style="italic", align="right",
         de="Das Binden\nder Schnüre,",
         en="Tying\nthe laces,"),
    dict(slice=57, seed=[312, 350, 880, 394], style="italic",
         de="das Falten der Ärmel",
         en="folding the sleeves"),
    dict(slice=58, seed=[370, 753, 1028, 925], style="italic",
         de="– alles saß. Ich fühlte\nmich so rein wie der\nStoff.",
         en="– everything fit. I felt as pure as the fabric."),
    dict(slice=59, seed=[382, 1080, 982, 1504], style="italic",
         de="Und so fromm und fein\nwie die Nähte am\nKorsett. Es war eine\nmillimetergenaue\n"
            "Maßanfertigung, die\nmeine androidenhafte\nGestalt glänzen ließ.",
         en="As pious and fine as the seams of the corset. Tailored to the millimeter, it made my "
            "android-like figure shine."),
    dict(slice=59, seed=[269, 1641, 1035, 1687], style="italic",
         de="Mein Einstieg in die Rolle!",
         en="My way into the role!"),
    dict(slice=60, seed=[108, 72, 936, 235], style="upright",
         de="Nein, eine Rolle war es nicht\nmehr. Die philosoph:innen\nstritten sich über Identität.",
         en="No, it was no longer a role. The philosophers were arguing about identity."),
    dict(slice=60, seed=[139, 1307, 1027, 1786], style="italic",
         de="Ja, da war die Spiritualität/\nTheologie\nüberlegen. Endlich durchströmte\nmich das am\n"
            "heutigen Tag so lange auf sich\nwarten lassende\nSelbstbewusstsein. Ich war\nbereit.",
         en="Yes, spirituality/theology had the upper hand there. At last the self-confidence I had waited "
            "for so long today flowed through me. I was ready."),
    dict(slice=62, seed=[109, 1743, 986, 1780], style="upright", kind="text",
         de="PLF schritt in den Flur hinaus.",
         en="PLF strode out into the hallway."),
    dict(slice=74, seed=[228, 1332, 883, 2008], style="upright", split=True,
         de="Im Jahr als Mao Zedong\nund Ulrike Meinhof zum\nHerren aufstiegen,\nkamen wir gemeinsam\n"
            "hinunter. Die hatten\nehrlich gesagt\neigentlich\npures Glück\nüberhaupt in den Himmel\n"
            "zu kommen. Schwer waren\nsie zu interpretieren.",
         en="In the year Mao Zedong and Ulrike Meinhof ascended to the Lord, we came down together. "
            "Honestly, they were pretty lucky to make it into heaven at all. They were hard to interpret."),
    dict(slice=77, seed=[167, 93, 1025, 391], style="upright",
         de="Mit doppeltem Blick betrachten\nmuss man solche Personen. Eine\nJugend ohne Gott. Selber\n"
            "wollten sie wohl zu Göttern\nwerden. Konsequent waren sie.",
         en="People like that deserve a second look. A youth without God. They probably wanted to become "
            "gods themselves. Consistent, they were."),
    dict(slice=80, seed=[143, 645, 1030, 934], style="italic", align="right",
         de="Das wäre schon eine tolle\nGeschichte, oder nicht? Science\nFiction passte damals einfach\n"
            "wahnsinnig gut zu meinem\nSelbstbild.",
         en="That would make a great story, wouldn't it? Back then, science fiction just fit my self-image "
            "insanely well."),
    dict(slice=82, seed=[206, 717, 801, 763], style="upright",
         de="Da bist du ja wieder?",
         en="There you are again?"),
    dict(slice=84, seed=[93, 1137, 626, 1237], style="upright",
         de="Hau ab! Zieh Leine!\nGeh du Untier!",
         en="Get lost! Beat it!\nBegone, you beast!"),
    dict(slice=85, seed=[150, 1620, 718, 1720], style="upright",
         de="Und hässlich bist du\nauch. OMG! Mon dieu!",
         en="And you're ugly too.\nOMG! Mon dieu!"),
    dict(slice=86, seed=[270, 1566, 1041, 2926], style="upright", split=True,
         de="KATZE zog sich zurück, als\nhätte es die Schatten der\nRealität unrechtmäßig\n"
            "überschritten, und wirbelte\ndabei eine Wolke aus Staub\nauf. In diesem Moment der\n"
            "Abwehr überkam PLF ein\nGefühl. Sie wurde erinnert\nan die dystopischen\nHeld:innen aus all den\n"
            "Erzählungen, von Sci-Fi bis\nhin zu den tiefen der\nheiligen Texte –\nHeld:innen, die zuerst\n"
            "gegen ihre inneren Dämonen\nkämpfen mussten, um in\neiner starren Welt ihre\n"
            "wahre Identität zu finden.\nNoch wusste PLF nicht, wie\nsehr dieser Kampf nun auch\n"
            "Teil ihrer eigenen Reise\nwerden würde.",
         en="CAT retreated, as if it had unlawfully crossed the shadows of reality, and stirred up a cloud of "
            "dust. In this moment of defense, a feeling came over PLF. They were reminded of the dystopian "
            "heroes of all the stories, from sci-fi to the depths of the holy texts – heroes who first had to "
            "fight their inner demons to find their true identity in a rigid world. PLF did not yet know how "
            "much this battle would now become part of their own journey."),
    dict(slice=88, seed=[186, 1419, 956, 1708], style="upright",
         de="PLF ging weiter und schritt\ndurch die mäandernden Wege\nder Siedlung wie Moses auf\n"
            "einem Hoverboard durchs\nRote Meer.",
         en="PLF walked on, striding through the meandering paths of the estate like Moses on a hoverboard "
            "through the Red Sea."),
    dict(slice=92, seed=[176, 831, 889, 1129], style="upright", kind="text", size=4.1, right=900,
         de="Wie ein Sonnenstrahl\nantwortete der Chor durch\ndie dichten Wolken mit\n„loud loud loud“, und es\n"
            "wurde langsam heller.",
         en="Like a ray of sunshine, the choir answered through the thick clouds with “loud loud loud”, and "
            "it slowly grew brighter."),
    dict(slice=94, seed=[256, 969, 912, 1447], style="upright", kind="text", align="center", size=4.1,
         rect=[60, 940, 1110, 1470],
         de="Ende des 1. Kapitels\n\n---\n\n2. Kapitel “ÄRGER”\n\nVeröffentlichungstermin\n1. November 2025",
         en="End of Chapter 1\n\n---\n\nChapter 2 “ANGER”\n\nRelease date\nNovember 1, 2025"),
]


def load_gray(n: int):
    img = np.array(Image.open(IMG.format(n)).convert("RGB"))
    return img, img.mean(axis=2)


def scan(line: np.ndarray, dark=100):
    """Index of the first dark pixel in ``line`` and the length of that dark run (None if no dark pixel)."""
    idx = np.where(line < dark)[0]
    if not len(idx):
        return None, 0
    i = int(idx[0])
    run = 0
    while i + run < len(line) and line[i + run] < dark and run < 12:
        run += 1
    return i, run


def outer_box(gray: np.ndarray, seed):
    """Find the outer edge of the box border around a text seed bbox. Returns [x0,y0,x1,y1] (exclusive x1/y1)."""
    x0, y0, x1, y1 = seed
    hh, ww = gray.shape
    border = 6

    def edge(values):
        vals = [v for v in values if v is not None]
        return int(np.median(vals)) if vals else None

    rows = [y for y in (y0 - 12, (y0 + y1) // 2, y1 + 12) if 0 <= y < hh]
    lefts, rights = [], []
    for y in rows:
        i, run = scan(gray[y, :max(x0 - 3, 0)][::-1])
        lefts.append(None if i is None else x0 - 3 - i - min(run, border))
        i, run = scan(gray[y, min(x1 + 3, ww):])
        rights.append(None if i is None else x1 + 3 + i + min(run, border))
    cols = [x for x in np.linspace(x0 + 5, x1 - 5, 7).astype(int)]
    tops, bottoms = [], []
    for x in cols:
        i, run = scan(gray[:max(y0 - 3, 0), x][::-1])
        tops.append(None if i is None else y0 - 3 - i - min(run, border))
        i, run = scan(gray[min(y1 + 3, hh):, x])
        bottoms.append(None if i is None else y1 + 3 + i + min(run, border))
    L, R, T, B = edge(lefts), edge(rights), edge(tops), edge(bottoms)
    return [0 if L is None else max(L + 1, 0), 0 if T is None else max(T + 1, 0),
            ww if R is None else min(R, ww), hh if B is None else min(B, hh)]


def ink_lines(gray: np.ndarray, box, dark=110):
    """Horizontal extents of text lines inside a box interior: list of (y0, y1, x0, x1)."""
    x0, y0, x1, y1 = box
    m = 14  # stay clear of the border
    sub = gray[y0 + m:y1 - m, x0 + m:x1 - m] < dark
    rows = sub.sum(axis=1) > 0
    out, start = [], None
    for y, on in enumerate(list(rows) + [False]):
        if on and start is None:
            start = y
        elif not on and start is not None:
            if y - start >= 8:
                cols = np.where(sub[start:y].any(axis=0))[0]
                out.append((y0 + m + start, y0 + m + y, x0 + m + int(cols[0]), x0 + m + int(cols[-1]) + 1))
            start = None
    return out


def font_size(de: str, lines, box_w=None, fallback=3.9):
    """Font size (% of slice width) from the glyph advance of the widest line; clamp so the longest line fits
    the caption box content width (1em padding each side + 2px@1x border)."""
    texts = [t for t in de.split("\n") if t.strip()]
    longest = max(texts, key=len)
    widths = sorted((x1 - x0 for y0, y1, x0, x1 in lines if y1 - y0 <= 70), reverse=True)
    size = None
    if widths and len(longest) >= 6:
        adv = widths[0] / (len(longest) - 0.3)
        size = adv / ADVANCE / W * 100
    if size is None or not (3.3 < size < 4.6):
        size = fallback
    if box_w:
        # content = box_w - 2*border(6px) - 2em ; needs len*0.6em (+1% slack)
        em_max = (box_w - 12) / (len(longest) * ADVANCE * 1.02 + 2)
        size = min(size, em_max / W * 100)
    return round(size, 2)


def pct_rect(x0, y0, x1, y1, yoff=0):
    return {"x": round(x0 / W * 100, 2), "y": round((y0 - yoff) / H * 100, 2),
            "width": round((x1 - x0) / W * 100, 2), "height": round((y1 - y0) / H * 100, 2)}


def layout_lines(text: str, chars: int) -> int:
    """Greedy word wrap in a monospace font: number of rendered lines at ``chars`` characters per line."""
    n = 0
    for para in text.split(chr(10)):
        words, cur = para.split(" "), 0
        n += 1
        for w in words:
            while len(w) > chars:          # overflow-wrap: anywhere
                if cur:
                    n += 1
                w, cur = w[chars:], chars
            need = len(w) if cur == 0 else cur + 1 + len(w)
            if need <= chars:
                cur = need
            else:
                n += 1
                cur = len(w)
    return n


def fit_en(d: dict, en: str, uppercase: bool):
    """For caption/narration boxes: check the English text fits the German box at fit >= 0.9. Single-line
    boxes that are too narrow are widened (the widened box still covers the baked one). Returns (rect, note)."""
    r = dict(d["rect"])
    em = d["size"] / 100 * W
    bw_px = 2 * W / 390                   # 2px border at a 390px column
    def capacity(width_pct, fit):
        cw = width_pct / 100 * W - 2 * bw_px - 2 * em * fit
        ch = r["height"] / 100 * H - 2 * bw_px - 1.6 * em * fit
        return int(cw // (ADVANCE * em * fit)), int((ch + 0.5) // (LINE_HEIGHT * em * fit))
    single = chr(10) not in en and capacity(r["width"], 0.95)[1] < 2
    for fit in (1.0, 0.975, 0.95, 0.925, 0.9):
        chars, rows = capacity(r["width"], fit)
        if single:
            # one-line boxes: the text must fit the width at full size, only the height may shrink a bit
            chars = capacity(r["width"], 1.0)[0]
        if layout_lines(en, chars) <= rows:
            return r, None if fit == 1.0 else f"fit {fit}"
    if single:
        need = (len(en) * ADVANCE * em * 1.02 + 2 * em + 2 * bw_px) / W * 100
        new_w = round(max(r["width"], need), 2)
        x = r["x"]
        if x + new_w > 99:
            x = round(max(1.0, 99 - new_w), 2)
        return {**r, "x": x, "width": new_w}, f"widened {r['width']} -> {new_w}"
    return r, "OVERFLOW"


def build():
    entries = []   # (host slice, dialog-without-text, de, en, id)
    counters = {}
    report = []
    for b in BOXES:
        n = b["slice"]
        kind = b.get("kind", "box")
        era = 3.85 if n <= 24 else 4.05  # slices 1-24 are lettered at a 60px line pitch, later ones at 63px
        img, gray = load_gray(n)
        yoff = 0
        host = n
        if b.get("split"):
            img2, gray2 = load_gray(n + 1)
            img = np.concatenate([img, img2])
            gray = np.concatenate([gray, gray2])
            host = n + 1
            yoff = H
        seed = b["seed"]
        box = outer_box(gray, seed)
        lines = ink_lines(gray, box)
        bgpx = img[box[1] + 8:box[3] - 8, box[0] + 8:box[2] - 8].reshape(-1, 3)
        bgpx = bgpx[bgpx.mean(axis=1) > 200]
        bg = np.median(bgpx, axis=0).astype(int) if len(bgpx) else np.array([255, 255, 255])
        bg_hex = "#%02x%02x%02x" % tuple(bg)
        mask = True if bg.min() >= 246 else bg_hex
        counters[host] = counters.get(host, 0) + 1
        did = f"s1c1-{host}-{counters[host]}"
        d = {"id": did}
        if kind == "box":
            size = b.get("size") or font_size(b["de"], lines, box[2] - box[0], era)
            d["rect"] = pct_rect(*box, yoff=yoff)
            d["variant"] = "narration" if b["style"] == "italic" else "caption"
            d["size"] = size
            if b.get("align"):
                d["align"] = b["align"]
            if b.get("uppercase"):
                d["uppercase"] = True
            d["mask"] = mask
            d["maskShape"] = "rect"
            d["pad"] = 0
        else:
            size = b.get("size") or font_size(b["de"], lines, None, era)
            if b.get("rect"):
                tx0, ty0, tx1, ty1 = b["rect"]
            else:
                tx0 = min(l[2] for l in lines) if lines else seed[0]
                ty0 = min(l[0] for l in lines) if lines else seed[1]
                tx1 = max(l[3] for l in lines) if lines else seed[2]
                ty1 = max(l[1] for l in lines) if lines else seed[3]
                # a little air around the glyphs, keep clear of the box border
                tx0, ty0 = max(tx0 - 6, box[0] + 10), max(ty0 - 8, box[1] + 10)
                tx1 = min(max(tx1 + 6, b.get("right", box[2] - 24)), box[2] - 12)
                ty1 = min(ty1 + 10, box[3] - 10)
            if b.get("align") != "center":
                # line boxes are 1.35em; glyph tops sit ~0.3em below the line box top in IBM Plex Mono
                em = size / 100 * W
                n_lines = b["de"].count(chr(10)) + 1
                glyph_top = b["rect"][1] if b.get("rect") else min(l[0] for l in lines)
                ty0 = glyph_top - 0.3 * em
                ty1 = ty0 + n_lines * LINE_HEIGHT * em + 0.1 * em
            d["rect"] = pct_rect(tx0, ty0, tx1, ty1, yoff=yoff)
            if b["style"] == "italic":
                # italic lettering straight on the art: frameless narration keeps the italic
                d["variant"] = "narration"
                d["frame"] = False
            else:
                d["variant"] = "bubble"
                d["font"] = "mono"
            d["size"] = size
            d["align"] = b.get("align", "left")
            d["valign"] = "top" if b.get("align") != "center" else "middle"
            if b.get("uppercase"):
                d["uppercase"] = True
            if b.get("rotation"):
                d["rotation"] = b["rotation"]
            d["mask"] = mask
            d["maskShape"] = "rect"
            d["pad"] = 0.3
        if kind == "box":
            en_rect, note = fit_en(d, b["en"], b.get("uppercase", False))
            if en_rect != d["rect"]:
                d["rect_en"] = en_rect
            if note:
                report.append(f"   en {did}: {note}")
        entries.append((host, d, b["de"], b["en"]))
        report.append(f"{did:10s} src s{n:02d} box {box} lines {len(lines)}/{len(b['de'].split(chr(10)))} "
                      f"size {d['size']} bg {bg_hex} {d['variant']}")
    return entries, report


def dialog_block(dialogs, lang, indent_unit, base_indent):
    """Serialize a dialog array: one compact JSON object per line, indented like the host file."""
    inner = base_indent + indent_unit
    rows = []
    for d, de, en in dialogs:
        obj = dict(d)
        en_rect = obj.pop("rect_en", None)
        if lang == "en" and en_rect:
            obj["rect"] = en_rect
        obj["text"] = de if lang == "de" else en
        # keep a stable, readable key order
        order = ["id", "rect", "text", "variant", "frame", "font", "size", "align", "valign", "uppercase", "rotation",
                 "mask", "maskShape", "pad"]
        obj = {k: obj[k] for k in order if k in obj}
        rows.append(inner + json.dumps(obj, ensure_ascii=False, separators=(", ", ": ")))
    return "[\n" + ",\n".join(rows) + "\n" + base_indent + "]"


def patch_chapter(path, lang, by_slice):
    with open(path, "r", encoding="utf-8", newline="") as f:
        src = f.read()
    nl = "\r\n" if "\r\n" in src else "\n"
    for n in sorted(by_slice):
        m = re.search(r'"image":\s*"images/3x/de/S1C1 - Slice %d\.jpg"' % n, src)
        if not m:
            raise SystemExit(f"{path}: slice {n} not found")
        # opening brace of the slice object
        open_i = src.rfind("{", 0, m.start())
        # matching closing brace (strings in this file contain no braces)
        depth, i = 0, open_i
        while True:
            c = src[i]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        close_i = i
        body = src[open_i:close_i + 1]
        # indentation of the "image" line = property indent
        line_start = src.rfind("\n", 0, m.start()) + 1
        prop_indent = src[line_start:m.start()]
        indent_unit = "\t" if prop_indent.startswith("\t") else "  "
        # drop an existing dialog property (re-run)
        body = re.sub(r',\s*"dialog":\s*\[.*?\n\s*\]', "", body, flags=re.S)
        block = dialog_block(by_slice[n], lang, indent_unit, prop_indent)
        # insert before the closing brace, after the last property
        last = body.rstrip()[:-1].rstrip()
        closing_ws = body[len(last):-1]
        body = last + "," + nl + prop_indent + '"dialog": ' + block.replace("\n", nl) + closing_ws + "}"
        src = src[:open_i] + body + src[close_i + 1:]
    json.loads(src)  # still valid JSON
    return src


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args(argv)
    entries, report = build()
    print("\n".join(report))
    by_slice = {}
    i18n = {}
    for host, d, de, en in entries:
        by_slice.setdefault(host, []).append((d, de, en))
        i18n[d["id"]] = {"de": de, "en": en, "slice": host}
    print(f"{len(entries)} dialogs on {len(by_slice)} slices")
    if args.dry_run:
        return 0
    os.makedirs(os.path.dirname(I18N), exist_ok=True)
    with open(I18N, "w", encoding="utf-8", newline="\n") as f:
        json.dump(i18n, f, ensure_ascii=False, indent="\t")
        f.write("\n")
    for lang, path in CHAPTERS.items():
        out = patch_chapter(path, lang, by_slice)
        with open(path, "w", encoding="utf-8", newline="") as f:
            f.write(out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
