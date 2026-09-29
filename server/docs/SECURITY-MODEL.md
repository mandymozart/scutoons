# Security Model for QR Code Signing

> **Paths since the server refactor (P1b):** `generate/files/` → `storage/files/` (`FILES_PATH`),
> `api/keys/keypair.json` → `storage/keys/keypair.json` (`KEYPAIR_PATH`), key setup pages → `tools/keys/`.
> The URLs are unchanged: `/api/keys/keys.php` (public key), `/generate/*.php` (legacy SPA), `/verify.html`; the admin
> API adds `/api/admin/keys` and `/api/admin/codes/{code}/signature`. Current docs: ../README.md, API.md, DEPLOY.md.

## Current Implementation: Client-Side Signing

### Why Client-Side?
ML-DSA-65 (FIPS 204 - Dilithium) is a **very new** post-quantum signature algorithm. As of 2025:
- ✅ Mature JavaScript implementation (noble-post-quantum)
- ❌ No mature PHP implementation available
- ❌ Server only runs PHP (no Node.js)

**Therefore: Client-side signing is the only practical option.**

## Security Layers

### 1. **htaccess Protection** 🔒
```apache
# /generate/.htaccess
AuthType Basic
AuthName "Protected Area"
AuthUserFile /path/to/.htpasswd
Require valid-user
```

- **Only authenticated users** can access `/generate/` folder
- Private key endpoint only works from this protected area
- Prevents public access to signing functionality

### 2. **Referer Check** 🔒
```php
// api/keys/keys.php
if (!str_contains($referer, $host . '/generate/')) {
    http_response_code(403);
    exit;
}
```

- Keys only served to requests from `/generate/` folder
- Prevents direct API access
- Additional layer beyond htaccess

### 3. **File Permissions** 🔒
```bash
chmod 600 /api/keys/keypair.json
```

- Private key file: Owner read/write only
- Not readable by web server group
- Protected from other users on shared hosting

### 4. **HTTPS Only** 🔒
- All communication encrypted in transit
- Private key never sent over plain HTTP
- TLS protects against network sniffing

## Security Model

```
┌─────────────────────────────────────────┐
│  Internet (Public)                      │
│  ❌ Cannot access /generate/            │
│  ❌ Cannot call /api/keys/keys.php      │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│  htaccess Authentication                │
│  Username + Password Required           │
└─────────────────────────────────────────┘
                 ↓
┌─────────────────────────────────────────┐
│  /generate/ folder (Protected)          │
│  ✅ Authenticated users only            │
│  ✅ Can load keys via API               │
│  ✅ Client-side signing with noble-pq   │
└─────────────────────────────────────────┘
```

## Risk Assessment

### ✅ **Acceptable Risks:**
1. **Private key in browser memory**
   - Only for authenticated users
   - Only during active signing session
   - Cleared when page closes

2. **Client-side signing**
   - Industry standard for post-quantum crypto (no PHP impl yet)
   - Many secure systems use client-side crypto (PGP, Signal, etc.)
   - The signed data is still cryptographically secure

### ⚠️ **Mitigated Risks:**
1. **Key theft from browser**
   - ✅ htaccess prevents unauthorized access
   - ✅ Referer check prevents direct API calls
   - ✅ HTTPS prevents network sniffing

2. **Unauthorized signing**
   - ✅ Must authenticate to access generate page
   - ✅ Audit trail in database (who signed what)
   - ✅ Timestamps in signatures

## Best Practices (Current Setup)

✅ **Implemented:**
- htaccess protection on `/generate/` folder
- Referer validation on key endpoint
- Restrictive file permissions (600) on keypair.json
- HTTPS only
- Keys cannot be regenerated without deleting existing keypair
- Audit trail (signatures in database + files)

## Future Improvements (When Available)

### Option 1: PHP Implementation
When ML-DSA-65 becomes available in PHP:
```php
// Future: Pure server-side signing
$signature = sodium_crypto_sign($message, $privateKey);
```

### Option 2: FFI to C Library
Use PHP FFI to call libdilithium:
```php
$ffi = FFI::cdef("...", "libdilithium.so");
$signature = $ffi->dilithium_sign(...);
```

### Option 3: Separate Signing Service
Run a dedicated Node.js signing microservice:
```
PHP App → HTTP POST → Node.js Signer → Returns signature
```

## Conclusion

**Current approach is secure given the constraints:**
- ✅ Multi-layer authentication
- ✅ Private key protected by htaccess + referer
- ✅ No public exposure of signing capability
- ✅ Post-quantum cryptographic security maintained
- ✅ Only practical solution without Node.js

**The security model is sound because:**
1. Attackers cannot access `/generate/` without credentials
2. Even with credentials, they can only sign as themselves (audit trail)
3. The cryptographic signatures are quantum-resistant
4. All generated certificates are verifiable by anyone (public key)

This is the **best possible security** given PHP-only hosting and the novelty of ML-DSA-65.
