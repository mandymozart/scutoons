Status: proposal (owner 2026-10-31: "the concept is good, but let's keep it under proposals for now") – direction: our own model on our content, TRIBE tested briefly on an A100 as an analysis tool in the story → webtoon tool chain; issue #202.

# TRIBE v2 → the reader's avatar mood (the Doom face)

**The idea (owner 2026-10-31):** the creator runs Meta's TRIBE v2 over a chapter offline; the predicted emotional
response per slice is stored with the chapter; the reader's avatar portrait shows that mood while the reader scrolls –
like Doom's status-bar face – with the mood sprites we already have (20 moods, `<char>_portrait-<mood>`). No model at
reading time.

## What TRIBE v2 is – and is not
- Meta FAIR, March 2026: predicts **fMRI responses** of an "average" subject to video, audio and text, per timestep, on
  the **fsaverage5 cortical mesh** (~20k vertices). Encoders: LLaMA 3.2-3B (text), V-JEPA 2 giant (video),
  Wav2Vec-BERT 2.0 (audio) → a Transformer → cortex. Zero-shot on new subjects; 2–3× earlier models (Meta's numbers).
- **It predicts brain activity, not emotions.** And only the **cortex** – no amygdala or other subcortex in the released
  weights (the repo trains a subcortical variant, not released as weights). An emotion has to be derived.
- **Compute: 28–32 GB VRAM** (LLaMA ~7 GB + V-JEPA 2 giant ~14 GB + w2v-BERT ~1 GB + TRIBE) – an A100 40 GB; a T4 (16 GB)
  runs out, our RTX 2070 (8 GB) is far off. Offline it is a rented GPU (RunPod / Colab A100) for minutes per chapter.
- **License: CC BY-NC 4.0 – non-commercial.** SCUTOONS sells memberships and merch, so using TRIBE's outputs in the
  product is a legal question (owner / a lawyer) before anything ships. LLaMA 3.2 is gated with its own license.

## From cortex to a mood
What the literature supports, roughly: **arousal is decodable** from cortical activity during films and generalises
across movies and people; **valence mostly is not** (a 2025 movie-watching study found a generalisable arousal
representation but none for valence). Insula, anterior cingulate and ventromedial prefrontal cortex carry consistent
signatures for felt states; the insula and auditory cortex track film-evoked emotion.

So a sound split:
- **Arousal from TRIBE**: an index from the predicted activity of the insula, the anterior cingulate and the
  auditory / visual association cortex (fsaverage5 ROIs via the repo's `utils_fmri` ROI tools), z-scored over the chapter.
- **Valence from the story itself**: the slice's text (bubbles, captions), its key visual and the music, judged by a
  language model in the creator (our generators already run there), or set by the creator by hand.
- **Arousal × valence → one of our 20 moods** on the circumplex, with ties broken by the scene's character:

| | negative | neutral | positive |
|---|---|---|---|
| **high arousal** | furious, angry, scared, disgusted, dizzy | surprised, confused | laughing, happy, tripping |
| **mid** | denial, bargaining, embarrassed | thinking, neutral | smug |
| **low arousal** | sad, crying, dying | neutral, acceptance | acceptance |

## The data and the reader
- **Per slice in the chapter document** (a draft field, published with the chapter): `feel: { mood, arousal, valence,
  source: "tribe" | "llm" | "creator" }`. The creator shows the curve over the chapter and lets the author override any
  slice – the author has the last word.
- **The reader's HUD**: the avatar portrait in a corner (the reader's own avatar character), its mood following the slice
  in view; held ≥ 1 s and cross-faded like the beings' heads (#188) so fast scrolling doesn't flicker; off in settings.
  A peak (high arousal) may pulse once – the Doom flinch.
- **Live** (later): the same curve can drive the beings' moods or a show's cues (#188, #190).

## Alternatives if the license says no
1. **The language model alone**: per slice, the model reads the text + looks at the key visual + knows the music and
   predicts the reader's arousal and valence. Commercially fine through the API we use, cheap, offline in the creator.
2. **Author tags**: the creator picks the mood per slice (or per row in the flow) – exact, no model.
3. TRIBE stays a **research comparison** (non-commercial): does the model's arousal curve agree with the language
   model's? Good material for an article, not for the product.

## The spike (after the license decision)
Chapter 2 (Cat Pope s1c2): slices → a reading-pace video (each slice scrolled through at ~0.8 s per screen), the bubbles
as text with timings, the chapter's music as audio → TRIBE on a rented A100 → the arousal index per slice → plotted
against the story (the sermon's crescendo should peak) → the moods → shown in the reader behind the draft preview.

## Sources
[Hugging Face facebook/tribev2 (model card, CC BY-NC 4.0)](https://huggingface.co/facebook/tribev2) ·
[GitHub facebookresearch/tribev2](https://github.com/facebookresearch/tribev2) ·
[AI at Meta announcement](https://x.com/AIatMeta/status/2037153756346016207) ·
[DataCamp TRIBE v2 tutorial (VRAM)](https://www.datacamp.com/tr/tutorial/tribe-v2-tutorial) ·
[Arousal generalises, valence not – PMC12058195](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12058195/) ·
[Mixed-valence feelings: insula / ACC / vmPFC – Cerebral Cortex 2024](https://www.citedrive.com/en/discovery/neural-patterns-associated-with-mixed-valence-feelings-differ-in-consistency-and-predictability-throughout-the-brain) ·
[Film-evoked emotion, insula + auditory – Sci Rep 2024](https://doaj.org/article/1dab6a61fd60425db3868633599c33ee) ·
[Phan et al. meta-analysis of emotion activation studies](https://pubmed.ncbi.nlm.nih.gov/15048050) ·
[YourStory on TRIBE v2](https://yourstory.com/ai-story/meta-tribe-v2-ai-model-human-thinking)

## Owner's direction (2026-10-31)
**A proposal for now, not a product feature.** The direction: **an analysis tool in our story → webtoon tool chain** –
our own model on our content (the language-model reading of the slices, option 1 above), and **TRIBE tested briefly on
a rented A100** to see what its arousal curve tells us about a chapter (where the tension rises, where it sags) next to
the storyboard / slice editor. Internal analysis, not shipped to readers; the avatar HUD in the reader waits until the
analysis proves useful. The license (CC BY-NC) still applies to anything that leaves the tool chain.
