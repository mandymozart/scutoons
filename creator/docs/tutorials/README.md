# Creator tutorials: the source of /docs

One Markdown file per use case of a creator, in one folder per phase of making a toon. The creator's public `/docs`
pages are built from these files (`creator/src/lib/docs`). Owner 2026-11-18 (#624): this is the baseline; the tutorials
are written for real once the first release's features are final and tested.

- `NN-phase/_group.md`: the phase's `key`, `title` and its intro line.
- `NN-phase/NN-slug.md`: one use case. The `NN-` prefixes set the order.
  - Front matter: `slug`, `title`, `summary` (no full stop), `who` (all · writer · artist · sound · performer · lead),
    `where` (creator · editor · deck), `often` (3 every session · 2 often · 1 now and then), `partly: true` when built only in part.
  - `## Facts`: `- Label: text` lines (optional).
  - `## Steps`: `1. …` – 3 to 6 steps, one or two sentences each, UI labels in “quotes” exactly as the app shows them.
    No steps = a template: the index lists it as "Guide soon" and no page is built for it.
  - `## Note`: one paragraph (optional).
