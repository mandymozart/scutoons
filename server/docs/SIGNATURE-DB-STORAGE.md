# Signature Storage - Database & File System

> **Paths since the server refactor (P1b):** `generate/files/` → `storage/files/` (`FILES_PATH`),
> `api/keys/keypair.json` → `storage/keys/keypair.json` (`KEYPAIR_PATH`), key setup pages → `tools/keys/`.
> The URLs are unchanged: `/api/keys/keys.php` (public key), `/generate/*.php` (legacy SPA), `/verify.html`; the admin
> API adds `/api/admin/keys` and `/api/admin/codes/{code}/signature`. Current docs: ../README.md, API.md, DEPLOY.md.

## Overview
QR code signatures are now stored in **two locations** for redundancy and flexibility:

1. **Database** (`qr.signature` JSON field)
2. **File System** (`files/signatures/SIGNATURE-{code}.json`)

## Architecture

### Database Storage
- **Table**: `qr`
- **Field**: `signature` (JSON with validation)
- **Contains**: Complete cryptographic signature package
  - `signature` - The ML-DSA signature bytes (hex)
  - `publicKey` - Public key for verification (hex)
  - `algorithm` - "ML-DSA-65"
  - `signedData` - **The exact JSON string that was signed** (CRITICAL!)
  - `timestamp` - When signature was created
- **Why store signedData?**
  - It's the exact canonical string that was cryptographically signed
  - Must match byte-for-byte for verification to succeed
  - Cannot be reconstructed (JSON order, date format, etc. matter)
  - Required for signature validation
- **Benefits**:
  - Faster retrieval for verification
  - Atomic operations with QR data
  - Easy to query and backup with database
  - No file system dependencies for verification

### File Storage
- **Location**: `generate/files/signatures/`
- **Format**: `SIGNATURE-{code}.json`
- **Benefits**:
  - Easy to download/share
  - Portable verification files
  - Can be distributed independently
  - Backwards compatible with existing system

## Flow

### Generation Flow
1. User generates QR code → `generate.php` creates DB record
2. Frontend generates post-quantum signature (ML-DSA-65)
3. Signature saved to **file system** → `files.php`
4. Signature saved to **database** → `save-signature.php`

### Verification Flow
1. User enters QR code on verify page
2. System tries **database first** → `get-signature.php`
3. If not found, **fallback to files** → `files/signatures/SIGNATURE-{code}.json`
4. Signature verified using ML-DSA-65 algorithm

## API Endpoints

### Save Signature to Database
```
POST /generate/save-signature.php
Content-Type: application/json

{
  "code": "ABC123",
  "signature": {
    "signature": "9bfc18c1a3ff...",
    "publicKey": "fa5efcb940ee...",
    "algorithm": "ML-DSA-65",
    "signedData": "{\"code\":\"ABC123\",\"type\":\"membership\",...}",
    "timestamp": "2025-10-19T01:04:07.688Z"
  }
}

// Note: signedData MUST be stored - it's the exact string that was signed
```

### Get Signature from Database
```
GET /generate/get-signature.php?code=ABC123

Response:
{
  "success": true,
  "code": "ABC123",
  "type": "membership",
  "signature": { ... },
  "fingerprint": "...",
  "created_at": "...",
  "expires_at": null
}
```

## Database Schema

```sql
ALTER TABLE qr ADD COLUMN signature LONGTEXT 
  CHARACTER SET utf8mb4 
  COLLATE utf8mb4_bin 
  DEFAULT NULL 
  CHECK (json_valid(signature));
```

## Why Dual Storage?

Digital signatures require the **exact original data** that was signed. The `signedData` field contains:
- Specific JSON key ordering
- Exact date format (ISO 8601 with milliseconds and timezone)
- Precise null representation
- No extra whitespace

**Attempting to reconstruct this would break verification** because:
```javascript
// Original (what was signed):
{"code":"ABC","type":"membership",...,"created_at":"2025-10-19T01:04:07.543Z",...}

// Reconstructed (would fail verification):
{"code":"ABC","type":"membership",...,"created_at":"2025-10-19T01:04:07+00:00",...}
// ❌ Different date format = different bytes = signature verification fails
```

## Benefits of Dual Storage

1. **Database**: Fast verification, integrated with QR data, easy queries
2. **Files**: Portable, shareable, independent backup, offline verification
3. **Redundancy**: Multiple copies ensure availability
4. **Flexibility**: Can verify using either method
5. **Backwards Compatibility**: Existing file-based verification continues working

## Future Considerations

- Could phase out file storage once database storage is proven stable
- Could add scheduled cleanup of old signature files
- Could implement signature archival for long-term storage
- Could add database replication for disaster recovery
