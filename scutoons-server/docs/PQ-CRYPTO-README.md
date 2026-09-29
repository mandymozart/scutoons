# Post-Quantum Cryptography Integration

> **Paths since the server refactor (P1b):** `generate/files/` → `storage/files/` (`FILES_PATH`),
> `api/keys/keypair.json` → `storage/keys/keypair.json` (`KEYPAIR_PATH`), key setup pages → `tools/keys/`.
> The URLs are unchanged: `/api/keys/keys.php` (public key), `/generate/*.php` (legacy SPA), `/verify.html`; the admin
> API adds `/api/admin/keys` and `/api/admin/codes/{code}/signature`. Current docs: ../README.md, API.md, DEPLOY.md.

This project uses **Noble Post-Quantum** cryptography library to provide quantum-resistant digital signatures for QR code certificates.

## Overview

- **Algorithm**: ML-DSA-65 (Dilithium) - FIPS 204 standard
- **Security Level**: NIST Level 3 (equivalent to 192-bit security)
- **Quantum Resistant**: Yes ✅
- **Library**: [@noble/post-quantum](https://github.com/paulmillr/noble-post-quantum)

## Installation

### Using Standalone Bundle (Current Implementation)

The system uses the standalone bundled version at:
```
/generate/app/lib/noble-post-quantum.min.js
```

Include in your HTML before any app scripts:

```html
<script src="app/lib/noble-post-quantum.min.js"></script>
```

This loads the library globally as `window.noblePostQuantum`.

**No NPM installation required!**

### Alternative: NPM/ES Modules (if preferred)

If you want to use ES modules instead:

```bash
npm install @noble/post-quantum
```

Then import in your code:
```javascript
import { ml_dsa65 } from '@noble/post-quantum/ml-dsa';
```

## How It Works

### 1. Key Generation

When the system starts, a keypair is generated (or loaded from secure storage):

```javascript
// Using standalone library
const ml_dsa65 = window.noblePostQuantum.ml_dsa65;
const seed = crypto.getRandomValues(new Uint8Array(32));
const secretKey = ml_dsa65.keygen(seed);
const publicKey = ml_dsa65.getPublicKey(secretKey);
```

**Important**: In production, generate keys ONCE and store them securely (e.g., HSM, encrypted storage, environment variables).

### 2. Signing QR Codes

Each QR code is automatically signed when generated:

```javascript
const signatureData = signQRCode(qrData);
// Returns: { signature, publicKey, algorithm, signedData, timestamp }
```

The signature covers:
- QR code unique identifier
- Type (membership, promo, event, xr)
- Redirect URL
- Creation timestamp
- Expiration (if any)

### 3. Certificate Integration

The post-quantum signature is automatically embedded in PDF certificates with:
- Algorithm name (ML-DSA-65)
- Timestamp of signing
- Cryptographic fingerprint
- Verification notice

### 4. Verification

Users can verify certificates at `/generate/verify.html` by pasting the signature export string.

## Security Considerations

### Key Management (Critical!)

**DO NOT** generate new keys on every application start. This breaks signature verification for existing certificates.

**Production Setup**:

1. **Generate once**: Run key generation once and store the keypair
2. **Secure storage**: Store in HSM, encrypted files, or environment variables
3. **Backup**: Keep secure backups of private keys
4. **Public key distribution**: Public key can be shared openly for verification

Example secure setup:

```javascript
// In production configuration
const PRIVATE_KEY = process.env.PQ_PRIVATE_KEY; // From secure env var
const PUBLIC_KEY = process.env.PQ_PUBLIC_KEY;

export function initializeKeypair() {
    if (PRIVATE_KEY && PUBLIC_KEY) {
        return {
            secretKey: hexToBytes(PRIVATE_KEY),
            publicKey: hexToBytes(PUBLIC_KEY)
        };
    }
    throw new Error('Keys not configured!');
}
```

### Why ML-DSA-65?

- **NIST approved**: Part of FIPS 204 standard
- **Battle-tested**: Based on Dilithium, one of the most analyzed PQ algorithms
- **Balanced**: Good compromise between signature size and security level
- **Future-proof**: Resistant to both classical and quantum attacks

### Signature Size

- **Signature**: ~3,293 bytes
- **Public Key**: ~1,952 bytes
- **Private Key**: ~4,000 bytes

These are larger than classical signatures but necessary for quantum resistance.

## Verification Flow

1. Certificate is generated with embedded signature
2. Signature export string is included in PDF
3. User can copy signature string
4. Visit verification page
5. Paste signature → automatic verification
6. See certificate details and validity

## API Reference

### `signQRCode(qrData)`
Signs QR code data with ML-DSA-65.

**Returns**: `{ signature, publicKey, algorithm, signedData, timestamp }`

### `verifySignature(signatureHex, publicKeyHex, signedData)`
Verifies a signature.

**Returns**: `boolean` - true if valid

### `generateFingerprint(signatureData)`
Creates human-readable fingerprint.

**Returns**: `string` - formatted as `XXXX:XXXX:XXXX:...`

### `exportSignature(signatureData)`
Exports signature as base64 string for sharing.

**Returns**: `string` - base64-encoded signature package

### `importSignature(base64Data)`
Imports signature from base64 string.

**Returns**: `Object` - signature data

## Browser Compatibility

- Modern browsers with WebCrypto API
- Chrome/Edge 88+
- Firefox 84+
- Safari 14+

## Resources

- [Noble Post-Quantum GitHub](https://github.com/paulmillr/noble-post-quantum)
- [NIST PQC Project](https://csrc.nist.gov/projects/post-quantum-cryptography)
- [FIPS 204 (ML-DSA)](https://csrc.nist.gov/pubs/fips/204/final)
- [Paul Miller's Cryptography Projects](https://paulmillr.com/noble/)

## Migration from Classical Signatures

If you're currently using ECDSA or RSA:

1. Generate ML-DSA keypair
2. Dual-sign for transition period (classical + PQ)
3. Gradually phase out classical signatures
4. Keep verifying old certificates with classical crypto

## License

This implementation uses the Noble Post-Quantum library (MIT License).

## Support

For issues with:
- **Noble library**: [GitHub Issues](https://github.com/paulmillr/noble-post-quantum/issues)
- **This integration**: Contact your development team
