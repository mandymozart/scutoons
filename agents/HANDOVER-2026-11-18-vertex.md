# Handover — Gemini switch to Vertex / vienna-struggle (2026-11-18)

For a fresh OpenCode session on SCUTOONS. This is state, not law: the rules are `AGENTS.md` and `agents/RULES.md` B10 (owner key `[vertex]`).

## Where we are
- Branch **`develop`**, local only, nothing pushed. Rule commit `c55cfceb` (AGENTS.md, RULES.md B10, decisions/law.md `[vertex]`).
- The switch is **done and verified**. OpenCode now uses the `google-vertex` provider backed by the `vienna-struggle` service account.
- Do not switch back to the `google` / AI Studio provider. Do not revoke the old `…oFtw` key without the owner's word.

## Why Vertex, not the AI Studio key
- The new AI Studio key `…KYPIrVg` authenticates (`models.list` 200, 50 models) but every real `generateContent` returns **HTTP 402 "prepayment credits are depleted"**. AI Studio prepay is a different billing path from the €269.35 Google Cloud free-trial credit in `vienna-struggle`.
- The `AQ…` key bound to `vertex-express@…` fails on `generativelanguage.googleapis.com` with `401 ACCESS_TOKEN_TYPE_UNSUPPORTED` even with the GA `roles/aiplatform.expressUser` and both API restrictions. It is not the route.
- Vertex AI via service-account ADC bills to the Cloud project and works.

## Working setup (verified)
- Service account: `vertex-express@vienna-struggle.iam.gserviceaccount.com`. JSON key stored at `C:\Users\Nutzer\.config\opencode\credentials\vertex-vienna-struggle.json` (ACL: user + SYSTEM only).
- OpenCode service env, set with `opencode service set env …` (each call restarts the service):
  - `GOOGLE_APPLICATION_CREDENTIALS` = the JSON key path
  - `GOOGLE_CLOUD_PROJECT` = `vienna-struggle`
  - `location: global` is in global config, not an env var.
- Check with `opencode service get`. CLI is **not on PATH**: `C:\Users\Nutzer\AppData\Roaming\ai.opencode.desktop\cli\2.1.4-x64\opencode-cli.exe`.
- Global `C:\Users\Nutzer\.config\opencode\opencode.json`: default `google-vertex/gemini-4-hydrogen`; `providers.google-vertex.settings` = project + location; five explicit model entries (Hydrogen, Carbon, Argon, 3.8 Flash, 3.5 Flash-Lite). `providers.google` is neutralised via `env: ["OPENCODE_DISABLED_GEMINI_KEY"]`; the old hardcoded key is gone.
- End-to-end proof: `opencode run --model google-vertex/gemini-3.5-flash-lite "Reply with exactly: OK"` → `OK`. Direct SDK test at `global` gave 32 prompt / 3 output / 13 thinking tokens.
- `google-vertex` exposes **45 models** (all Gemini models plus Vertex-hosted Claude / Grok / Mistral), so `/models` shows the full catalog. The plain `models` list hides older families; use "show all" in the picker.

## Model selection in OpenCode
- `/models` → pick `google-vertex/<model>` for the session. Session-only; it does not rewrite the root default.
- Ladder from RULES B10 (efforts/caps from `C:\Development\agents\models.json`):
  - orchestrator → `gemini-4-hydrogen` medium
  - code.S → `gemini-3.5-flash-lite` medium
  - code.M → `gemini-4-hydrogen` high
  - code.L → `gemini-4-carbon` high
  - code.review → `gemini-4-hydrogen` high
  - code.design → `gemini-4-argon` high

## Still open
- **Cost booking is done, and the sink is the remote API — not this repo.** Both setup entries were pushed to `POST /api/usage` (`https://agents.porschuetz.de`, remote DB behind `C:\Development\agents`) and verified there: `session-gemini-setup-20261118` $0.0919, `session-vertex-switch-20261118` $0.0001. The temporary entries in `scutoons/agents/costs.json` were removed again; do not log costs into the SCUTOONS repo ledger. Use `agents/gemini-usage.mjs` (API-only, `project: 'scutoons'`) or `python C:\Development\agents\costs\gemini_usage.py --project scutoons …`; price registry `C:\Development\agents\prices.json`, tests `python C:\Development\agents\costs\tests\run.py` → 13/13. Gemini mapping: `service: "gemini"`, tiers `std`/`batch`/`flex`/`priority`, `input = total_input_tokens - total_cached_tokens`, `cache_read = total_cached_tokens`, cache writes 0, `output = total_output_tokens`, thinking informational only. One entry per session (the API replaces that session's rows).
- **Global ladder**: `C:\Development\agents\models.json` still maps every kind to Claude; the Gemini mapping above is currently only a SCUTOONS rule. Ask before changing the shared file.
- **SCUTOONS handoff from Claude**: confirm `deck:ci` on local main after #326; review → rebase → check → locally merge #622 (SDK sections worktree) then #621 (camera move repeat worktree), never push; #623 editor crash `0xC0000409` with a full scene of video layers plus a character; browser deck runtime still blocked by #290 / #306.
- Local `main` is ahead of `origin/main` by 183 commits. Nothing has been pushed.

## Do not
- Do not use `…KYPIrVg` for generation (402) and do not restore the old hardcoded `…oFtw` key in config.
- Do not revoke any key without explicit owner confirmation.
- Do not push.
