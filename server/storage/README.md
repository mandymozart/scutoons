# storage/ – private files of this installation (never served, not in git)

| Folder | What | Setting |
|---|---|---|
| `files/batch/` | batch manifests `BATCH-<timestamp>-<edition>.json` | `FILES_PATH` |
| `files/svgs/`, `files/certificates/`, `files/signatures/`, `files/zips/` | QR SVGs, certificate PDFs, signature JSON, batch ZIPs | `FILES_PATH` |
| `migration-backups/` | the rows each migration run through the API changed, before it changed them (JSON lines, one file per run – BACKLOG 10.61) | `MIGRATION_BACKUP_PATH` |
| `storyboard-work/<toon>/<chapter>/` | the storyboard's shared work storage: attempts members share for review (#120; creators only, emptied per chapter after the release) | `STORYBOARD_WORK_PATH` |
| `keys/keypair.json` | ML-DSA-65 keypair of the certificate signatures (PRIVATE – back it up outside the server) | `KEYPAIR_PATH` |

The deploy never uploads or deletes anything here. Before the refactor these lived in `generate/files/` and
`api/keys/keypair.json` (see docs/DEPLOY.md "Cutover").
