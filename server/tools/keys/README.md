# Signing keys (ML-DSA-65) – one-time setup tools

The certificates of authenticity are signed client-side (noble-post-quantum in the admin tools) with one ML-DSA-65
keypair. The server keeps it in **`storage/keys/keypair.json`** (setting `KEYPAIR_PATH`; private, never served, not in
git, not deployed – upload it by FTP once) and hands out:

- the **public key** to everyone: `GET /api/keys/keys.php` (printed URL, verify page, third-party checks);
- the **full keypair** to admins only: `GET /api/admin/keys` (role admin) and the legacy `generate/keys.php` (token).

**Generate a keypair ONLY ONCE** – a new key invalidates every signature printed so far. Production already has one;
back it up outside the server (it was `api/keys/keypair.json` before the refactor).

## Generating a keypair (only for a new installation)

These pages are not deployed. Run them locally from the server folder:

```bash
php -S localhost:9200        # document root = the server folder
# open http://localhost:9200/tools/keys/generate-keys.php → "Generate" → writes storage/keys/keypair.json
```

`save-keys.php` refuses to overwrite an existing keypair. Then upload `storage/keys/keypair.json` to the same path on
the server (permissions 600 where the host allows it) and check `/api/keys/keys.php`.

`keypair.json` = `{ privateKey (hex, 4032 bytes), publicKey (hex, 1952 bytes), algorithm: "ML-DSA-65", generated }`.
More background: docs/KEY-MANAGEMENT.md, docs/SECURITY-MODEL.md, docs/VERIFICATION-GUIDE.md.
