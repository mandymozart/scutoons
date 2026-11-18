# SCUTOONS — OpenCode entry point

OpenCode sessions follow the same law book as every other agent. Start with `CLAUDE.md` (it gives the start-up order), then `agents/RULES.md` (the law book) and `agents/HANDOVER.md` (the current state). Keep context small: grep and targeted reads, never whole files.

## Model selection while using OpenCode

As of 2026-11-18 (owner decision `[vertex]`), every OpenCode session on SCUTOONS takes its model from the **`google-vertex`** provider, billed to Google Cloud project `vienna-struggle` through the `vertex-express@vienna-struggle.iam.gserviceaccount.com` service account (`GOOGLE_APPLICATION_CREDENTIALS` via ADC). Do **not** use the `google` / AI Studio API-key provider for SCUTOONS sessions.

Pick the model by the kind ladder in `agents/RULES.md` **B10** (orchestrator → Hydrogen, small → Flash-Lite, frontier → Carbon/Argon). Select it for the session with `/models` as `google-vertex/<model>`. Changing the model for one session never rewrites the root default.
