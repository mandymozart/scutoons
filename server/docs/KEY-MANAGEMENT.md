# 🔐 Key Management Guide

> **Paths since the server refactor (P1b):** `generate/files/` → `storage/files/` (`FILES_PATH`),
> `api/keys/keypair.json` → `storage/keys/keypair.json` (`KEYPAIR_PATH`), key setup pages → `tools/keys/`.
> The URLs are unchanged: `/api/keys/keys.php` (public key), `/generate/*.php` (legacy SPA), `/verify.html`; the admin
> API adds `/api/admin/keys` and `/api/admin/codes/{code}/signature`. Current docs: ../README.md, API.md, DEPLOY.md.

## Understanding Keys

### Private Key (Secret) 🔒
**Who has it**: ONLY YOU (the certificate issuer)  
**Purpose**: Signs new certificates  
**Never share**: ❌ Never give to users  
**Storage**: Secure server storage, encrypted  

### Public Key (Shareable) 🔓
**Who has it**: EVERYONE (users, public)  
**Purpose**: Verifies signatures  
**Freely share**: ✅ Include in PDFs, signatures, website  
**Storage**: Public files, included in signature data  

---

## Analogy: Like a Wax Seal

**Private Key** = Your personal wax seal stamp  
- Only YOU have it
- You use it to seal (sign) documents
- Proves documents came from you

**Public Key** = The wax seal impression  
- Everyone can see it
- Anyone can verify it came from your seal
- Can't be used to create new seals

**Signature** = The wax on the document  
- Proves you sealed (signed) it
- Can't be forged without your stamp

---

## What Users Get

### ✅ In the PDF Certificate:
```
┌─────────────────────────────────────┐
│ Certificate of Authenticity         │
│                                     │
│ QR Code: yB56cs59                   │
│ Fingerprint: ABCD:1234:5678...      │
│                                     │
│ To verify:                          │
│ 1. Visit verify.html                │
│ 2. Enter code: yB56cs59             │
│                                     │
│ Public Key: fed30b8a...             │
│ (This is safe to share!)            │
└─────────────────────────────────────┘
```

### ✅ In Signature File (files/signatures/):
```json
{
  "signature": "hex...",      ← Cryptographic proof
  "publicKey": "hex...",      ← For verification (safe!)
  "signedData": "{...}",      ← What was signed
  "algorithm": "ML-DSA-65"    ← How it was signed
}
```

### ❌ Users NEVER Get:
- Private key
- Seed used to generate keys
- Any secret material

---

## Current Setup

### Where Keys Are Generated

**File**: `app/utils/crypto.js`

```javascript
export function initializeKeypair() {
    if (!KEYPAIR) {
        // This generates ONCE per page session
        const seed = crypto.getRandomValues(new Uint8Array(32));
        const secretKey = ml_dsa65.keygen(seed);
        const publicKey = ml_dsa65.getPublicKey(secretKey);
        
        KEYPAIR = { secretKey, publicKey };
        // ↑ secretKey = PRIVATE (keep secret!)
        // ↑ publicKey = PUBLIC (share freely!)
    }
    return KEYPAIR;
}
```

**Current Behavior**:
- 🔴 **Development Mode**: New keys generated on every page load
- ⚠️ **Problem**: Old signatures can't be verified after refresh
- ✅ **Solution**: Use persistent keys (see below)

---

## Production Setup (Important!)

### Step 1: Generate Keys ONCE

Run this in browser console **ONE TIME**:

```javascript
// Open generate page in browser
// Open console (F12)
// Run this:

const ml_dsa65 = window.noblePostQuantum.ml_dsa65;
const seed = crypto.getRandomValues(new Uint8Array(32));
const secretKey = ml_dsa65.keygen(seed);
const publicKey = ml_dsa65.getPublicKey(secretKey);

// Convert to hex for storage
const secretKeyHex = Array.from(secretKey)
    .map(b => b.toString(16).padStart(2, '0')).join('');
const publicKeyHex = Array.from(publicKey)
    .map(b => b.toString(16).padStart(2, '0')).join('');

console.log('SAVE THESE SECURELY:');
console.log('\n=== PRIVATE KEY (SECRET!) ===');
console.log(secretKeyHex);
console.log('\n=== PUBLIC KEY (SHAREABLE) ===');
console.log(publicKeyHex);
```

### Step 2: Store Securely

**Option A: Environment Variables** (Simple)

Create `.env` file (add to `.gitignore`!):
```env
PQ_PRIVATE_KEY=your_secret_key_hex_here
PQ_PUBLIC_KEY=your_public_key_hex_here
```

**Option B: Encrypted File** (Better)

```bash
# Save private key encrypted
echo "your_secret_key_hex" | openssl enc -aes-256-cbc -pbkdf2 -out private.key.enc

# Load in PHP (example)
$encrypted = file_get_contents('private.key.enc');
$privateKey = openssl_decrypt($encrypted, 'aes-256-cbc', $_ENV['KEY_PASSWORD']);
```

### Step 3: Update crypto.js

**Replace the initializeKeypair function:**

```javascript
export function initializeKeypair() {
    if (!KEYPAIR) {
        // In production: load from server endpoint
        // For now: hardcode (ONLY public key is somewhat OK)
        
        const publicKeyHex = 'YOUR_PUBLIC_KEY_HEX_HERE';
        
        // Private key should come from secure server-side signing
        // For client-side signing (current setup), you need to store it securely
        // WARNING: Storing private key client-side is NOT recommended for production!
        
        const secretKeyHex = 'LOADED_SECURELY_FROM_SERVER';
        
        KEYPAIR = {
            secretKey: hexToBytes(secretKeyHex),
            publicKey: hexToBytes(publicKeyHex)
        };
    }
    return KEYPAIR;
}
```

**Better Approach**: Move signing to server-side PHP!

---

## Better Architecture: Server-Side Signing

### Why?

- ✅ Private key never exposed to browser
- ✅ More secure
- ✅ Easier key management
- ✅ Can use hardware security modules (HSM)

### Implementation:

**1. Create PHP Signing Endpoint**

`api/sign.php`:
```php
<?php
// Load private key securely
$privateKeyHex = $_ENV['PQ_PRIVATE_KEY'];

// Get data to sign
$data = json_decode(file_get_contents('php://input'), true);

// Call noble-post-quantum via Node.js subprocess or use PHP FFI
// (noble is JavaScript library, need bridge to PHP)

// Return signature
echo json_encode([
    'signature' => $signatureHex,
    'publicKey' => $publicKeyHex,
    'timestamp' => time()
]);
```

**2. Update generate.js**

```javascript
// Instead of signing client-side:
const signatureData = signQRCode(this.currentQR);

// Call server:
const response = await fetch('/api/sign.php', {
    method: 'POST',
    body: JSON.stringify(this.currentQR)
});
const signatureData = await response.json();
```

---

## Quick Test

After fixing files.php, generate a QR code and check:

```bash
# Should see these folders created:
ls -la generate/files/
# certificates/
# qrs/
# svgs/
# signatures/   ← Should be here now!

# Check if signature was saved:
ls -la generate/files/signatures/
# SIGNATURE-yB56cs59.json   ← Your signature file!
```

Then verify:
1. Open `generate/verify.html`
2. Enter your QR code
3. Should fetch and verify the signature ✅

---

## Summary

| Item | User Gets | You Keep |
|------|-----------|----------|
| **Private Key** | ❌ Never | 🔒 Secret storage |
| **Public Key** | ✅ In PDFs/signatures | 🔓 Shareable |
| **Signature** | ✅ Proof of authenticity | 📝 Audit log |
| **QR Code** | ✅ Identifier | 📊 Database |

**Remember**: Private key = Your secret. Public key = Everyone can have it!
