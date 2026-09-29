# storage/ – private files of this installation (never served, not in git)

| Folder | What | Setting |
|---|---|---|
| `files/batch/` | batch manifests `BATCH-<timestamp>-<edition>.json` | `FILES_PATH` |
| `files/svgs/`, `files/certificates/`, `files/signatures/`, `files/zips/` | QR SVGs, certificate PDFs, signature JSON, batch ZIPs | `FILES_PATH` |
| `keys/keypair.json` | ML-DSA-65 keypair of the certificate signatures (PRIVATE – back it up outside the server) | `KEYPAIR_PATH` |

The deploy never uploads or deletes anything here. Before the refactor these lived in `generate/files/` and
`api/keys/keypair.json` (see docs/DEPLOY.md "Cutover").
