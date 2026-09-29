# 🔍 Signature Verification Guide

> **Paths since the server refactor (P1b):** `generate/files/` → `storage/files/` (`FILES_PATH`),
> `api/keys/keypair.json` → `storage/keys/keypair.json` (`KEYPAIR_PATH`), key setup pages → `tools/keys/`.
> The URLs are unchanged: `/api/keys/keys.php` (public key), `/generate/*.php` (legacy SPA), `/verify.html`; the admin
> API adds `/api/admin/keys` and `/api/admin/codes/{code}/signature`. Current docs: ../README.md, API.md, DEPLOY.md.

## How Verification Works

### Overview

```
┌─────────────┐
│   User has  │
│     PDF     │
└──────┬──────┘
       │
       │ QR Code: d6VXJEHa
       ↓
┌──────────────┐
│ verify.html  │
└──────┬───────┘
       │
       │ fetch('/files/signatures/SIGNATURE-d6VXJEHa.json')
       ↓
┌─────────────────┐
│  Signature File │ Contains:
│   (Server)      │ - Signature (hex)
└──────┬──────────┘ - Public Key (hex)
       │             - Signed Data (JSON)
       │             - Timestamp
       │
       │ Load noble-post-quantum library
       ↓
┌──────────────────┐
│  ML-DSA-65      │
│  Verification   │  verify(publicKey, message, signature)
└──────┬───────────┘
       │
       ↓
┌────────────┐
│   Result   │  ✅ Valid or ❌ Invalid
└────────────┘
```

## Step-by-Step Process

### 1. User Action
User enters QR code (e.g., `d6VXJEHa`) in verify.html

### 2. Fetch Signature
```javascript
const response = await fetch(`files/signatures/SIGNATURE-${code}.json`);
const sigData = await response.json();
```

**What's in the signature file:**
```json
{
  "code": "d6VXJEHa",
  "signature": {
    "signature": "hex_string_6586_chars",    // The cryptographic signature
    "publicKey": "hex_string_3904_chars",    // Public key for verification
    "signedData": "{...JSON...}",            // What was signed
    "algorithm": "ML-DSA-65",                // Signature algorithm
    "timestamp": "2025-10-18T22:44:48.042Z"  // When it was signed
  },
  "fingerprint": "48F0:4A0D:...",            // Human-readable fingerprint
  "created_at": "2025-10-18T22:44:47.965Z"   // When QR was created
}
```

### 3. Convert Hex to Bytes
```javascript
const signature = hexToBytes(sigData.signature.signature);    // 3293 bytes
const publicKey = hexToBytes(sigData.signature.publicKey);    // 1952 bytes
const message = new TextEncoder().encode(sigData.signature.signedData);
```

### 4. Cryptographic Verification
```javascript
const ml_dsa65 = window.noblePostQuantum.ml_dsa65;
const isValid = ml_dsa65.verify(publicKey, message, signature);
```

**This checks:**
- Was this signature created by the holder of the private key?
- Does the signature match the exact data that was signed?
- Has the data been tampered with?

### 5. Display Result
- ✅ **Valid**: Certificate is authentic
- ❌ **Invalid**: Certificate has been tampered with or signature doesn't match

---

## What Gets Signed?

When a QR code is generated, this exact data is signed:

```json
{
  "code": "d6VXJEHa",
  "type": "membership",
  "redirect_to": "https://scutoons.com/catpope",
  "url": "https://qr.scutoons.com?c=d6VXJEHa",
  "created_at": "2025-10-18T22:44:47.965Z",
  "expires_at": null
}
```

**Why JSON?** This creates a canonical (unchangeable) format. Any change (even a single character) will make the signature invalid.

---

## Security Properties

### What The Signature Proves

✅ **Authenticity**: This QR code was created by the holder of the private key (you/SCUTOONS)  
✅ **Integrity**: The data has not been modified since signing  
✅ **Non-repudiation**: The signer cannot deny creating this signature  
✅ **Quantum Resistance**: Remains secure even against quantum computers  

### What The Signature Does NOT Prove

❌ **When verified**: Just proves it was signed at the timestamp  
❌ **Who verified**: Anyone can verify, no identity check  
❌ **QR still valid**: Doesn't check expiration (handled separately)  

---

## Why It Might Fail

### Common Reasons for "Invalid Signature"

#### 1. **Keys Don't Match**
```
Problem: Different private key was used to sign than the public key used to verify
Solution: Ensure keypair.json hasn't changed since QR was generated
```

#### 2. **Data Was Modified**
```
Problem: The signed data in the JSON doesn't match what was originally signed
Solution: This is actually working correctly - the certificate was tampered with!
```

#### 3. **Wrong Signature Format**
```
Problem: Hex string is corrupted or truncated
Solution: Re-download the signature file or regenerate the QR code
```

#### 4. **Library Not Loaded**
```
Problem: noble-post-quantum.min.js failed to load
Solution: Check browser console, refresh page, check file exists
```

#### 5. **Wrong Key Used**
```
Problem: Keys were regenerated after this signature was created
Solution: Restore old keypair.json or accept that old signatures are invalid
```

---

## Debugging Verification Issues

### 1. Check Library Loaded

Open browser console:
```javascript
window.noblePostQuantum
// Should show: {ml_dsa65: {...}, ...}

window.noblePostQuantum.ml_dsa65
// Should show: {keygen: ƒ, sign: ƒ, verify: ƒ, ...}
```

### 2. Check Signature File Exists

```bash
ls generate/files/signatures/SIGNATURE-d6VXJEHa.json
# Should exist and be ~8-12 KB
```

### 3. Check Signature Format

```javascript
// In browser console after fetching signature
console.log('Signature length:', sigData.signature.signature.length);
// Should be: 6586 (hex chars)

console.log('Public key length:', sigData.signature.publicKey.length);
// Should be: 3904 (hex chars)
```

### 4. Manual Verification Test

```javascript
// Copy this into browser console on verify.html

const testSignature = "your_signature_hex_here";
const testPublicKey = "your_public_key_hex_here";
const testData = '{"code":"d6VXJEHa",...}';

// Convert to bytes
const hexToBytes = (hex) => {
    const bytes = new Uint8Array(hex.length / 2);
    for (let i = 0; i < bytes.length; i++) {
        bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }
    return bytes;
};

const sig = hexToBytes(testSignature);
const pubKey = hexToBytes(testPublicKey);
const msg = new TextEncoder().encode(testData);

const ml_dsa65 = window.noblePostQuantum.ml_dsa65;
const result = ml_dsa65.verify(pubKey, msg, sig);

console.log('Manual verification result:', result);
```

### 5. Check Console Logs

When you click "Verify by Code", check console for:
```
✅ noble-post-quantum library loaded successfully
Loaded signature data: {...}
Verifying with:
- Signature length: 6586
- Public key length: 3904
- Signed data: {"code":"d6VXJEHa",...}
Verification result: true (or false)
```

---

## File Locations

### Verification Page
```
/generate/verify.html
```

### Signature Files
```
/generate/files/signatures/SIGNATURE-{CODE}.json
```

### Cryptography Library
```
/generate/app/lib/noble-post-quantum.min.js
```

### Crypto Utilities
```
/generate/app/utils/crypto.js
```

### Server Keys
```
/api/keys/keypair.json  (contains private & public key)
```

---

## Download Signature File

Users can download the signature JSON file:

1. Verify the certificate successfully
2. Click "📥 Download Signature File" button
3. Save `SIGNATURE-{CODE}.json` to their device

**Use cases:**
- Offline verification (with library cached)
- Share proof of authenticity
- Archive for records
- Advanced verification tools

---

## Verification Methods

### Method 1: By Code (Easiest)
1. Enter QR code: `d6VXJEHa`
2. System fetches signature from server
3. Automatic verification

**Pros:**
- Simple for users
- No file handling
- Always uses server's copy

### Method 2: By Full Signature (Advanced)
1. Paste base64 signature export
2. System imports and verifies
3. Works offline (if library cached)

**Pros:**
- No server dependency
- Works offline
- User controls the data

---

## Security Best Practices

### For Users
✅ Verify certificates before trusting them  
✅ Download signature files for important certificates  
✅ Check fingerprint matches what's on the PDF  
❌ Don't trust screenshots or copies - verify the original  

### For Administrators
✅ Keep `keypair.json` backed up securely  
✅ Never regenerate keys unless absolutely necessary  
✅ Monitor signature generation logs  
✅ Keep noble-post-quantum library updated  
❌ Never expose private key to untrusted users  

---

## API for Verification

You can verify signatures programmatically:

```javascript
import { verifySignature } from './app/utils/crypto.js';

// Load signature from server
const response = await fetch(`/files/signatures/SIGNATURE-${code}.json`);
const sigData = await response.json();

// Verify
const isValid = verifySignature(
    sigData.signature.signature,
    sigData.signature.publicKey,
    sigData.signature.signedData
);

console.log('Is valid?', isValid);
```

---

## Next Steps

1. ✅ Generate a QR code
2. ✅ Check signature file created: `/files/signatures/SIGNATURE-{CODE}.json`
3. ✅ Visit `verify.html` and enter the code
4. ✅ Check browser console for detailed logs
5. ✅ If invalid, check the debugging steps above

**Still having issues?** Check:
- Is `noble-post-quantum.min.js` loaded?
- Does the signature file exist?
- Has `keypair.json` been changed since signing?
- Check browser console for specific errors
