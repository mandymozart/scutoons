# Workflows

Repeatable production workflows (human + AI tools). One folder per domain; each workflow is a markdown file with the
spec, the prompts and the steps. They are the manual/semi-automated precursors of the creator's automation
(see `agents/AUTOMATION.md`) and grow as we have more data to work with.

| Folder | Workflow | What it does |
|---|---|---|
| `characters/` | [base-views.md](characters/base-views.md) | any 1 of the 4 base views (portrait, avatar, full, A-pose) → all 4, normalized, named, registered |
| `items/` | [item-icons.md](items/item-icons.md) | virtual item icons (camera, cucumber, …) in the character render style (image generator or designer) |
| `scenes/` | [generate-shots.md](scenes/generate-shots.md) | script scene → approved, character-consistent shot stills (FLORA) for the Figma collage |
| `scenes/` | [storyboard.md](scenes/storyboard.md) + [storyboard-taste.md](scenes/storyboard-taste.md) | script scene → storyboard (stacked shots, light, perspective) → Sin City frames, chapter draft, reference packs (agents/STORYBOARD.md) |
| `storyboard-studio/` | [README.md](storyboard-studio/README.md) | the layers of a chapter side by side, frames edited by hand with a live render, variants, reviews (`npm run dev:studio`) |

Conventions: artist-made content always wins over generated content and may replace it at any time; every workflow
ends in the same place (files on the CDN by convention + registered data), so its output is usable in the creator.
