# Quick Setup: Post-Quantum Signatures

## 1. Include Standalone Library

The system uses the standalone (bundled) version of noble-post-quantum at:
```
/generate/app/lib/noble-post-quantum.min.js
```

This is already included in the HTML files:
```html
<script src="app/lib/noble-post-quantum.min.js"></script>
```

No NPM installation needed! The library is loaded globally as `window.noblePostQuantum`.

## 2. Configuration (Important!)

### For Development/Testing
The system will auto-generate a keypair on first use. Check the console for the public key.

### For Production (Critical!)
**DO NOT use auto-generated keys in production!**

Generate keys once and store them securely:

```javascript
// Run this ONCE to generate production keys (in browser console)
const ml_dsa65 = window.noblePostQuantum.ml_dsa65;

const seed = crypto.getRandomValues(new Uint8Array(32));
const secretKey = ml_dsa65.keygen(seed);
const publicKey = ml_dsa65.getPublicKey(secretKey);

console.log('Private Key:', Array.from(secretKey).map(b => b.toString(16).padStart(2, '0')).join(''));
console.log('Public Key:', Array.from(publicKey).map(b => b.toString(16).padStart(2, '0')).join(''));
```

Store these in environment variables or secure key management:

```env
PQ_PRIVATE_KEY=your_private_key_hex_here
PQ_PUBLIC_KEY=your_public_key_hex_here
```

Then update `app/utils/pqCrypto.js` to load from env vars instead of generating.

## 3. Verify Library is Loaded

Open browser console and check:

```javascript
console.log(window.noblePostQuantum); // Should show the library object
console.log(window.noblePostQuantum.ml_dsa65); // Should show ML-DSA functions
```

If undefined, make sure `noble-post-quantum.min.js` is loaded before your app scripts.

## 4. Test It!

1. Generate a QR code at `/generate/index.php`
2. See the signature fingerprint displayed
3. Download the Certificate of Authenticity PDF
4. Note the "Post-Quantum Cryptographic Verification" section
5. Visit `/generate/verify.html` to verify signatures

## 5. Features Enabled

✅ **Automatic Signing**: Every QR code is signed with ML-DSA-65
✅ **PDF Integration**: Certificates include signature metadata
✅ **UI Display**: Signature fingerprint shown in the interface
✅ **Verification Page**: Standalone verification at `/generate/verify.html`
✅ **Console Logging**: Full signature export in browser console

## Architecture

```
User generates QR
     ↓
QR data signed with ML-DSA-65
     ↓
Signature embedded in certificate PDF
     ↓
Signature export logged to console
     ↓
User can verify anytime at verify.html
```

## What's Protected?

The signature covers:
- QR code unique identifier (`code`)
- Type (`membership`, `promo`, `event`, `xr`)
- Redirect URL (`redirect_to`)
- Full QR URL (`url`)
- Creation timestamp (`created_at`)
- Expiration timestamp (`expires_at`)

## Verification

Anyone can verify a certificate:
1. Open the certificate PDF
2. Copy the signature export string (from console or PDF)
3. Visit `/generate/verify.html`
4. Paste and verify

## Security Notes

- **Private Key**: Keep secret, never expose in client code
- **Public Key**: Can be distributed openly for verification
- **Signatures**: Safe to share, prove authenticity
- **Quantum Safe**: Secure against quantum computer attacks
- **NIST Approved**: ML-DSA-65 is FIPS 204 standard

## Next Steps

1. **Key Management**: Set up secure key storage
2. **Backup**: Create secure backups of private keys
3. **Monitoring**: Log signature generation for audit trail
4. **Documentation**: Document your public key for users
5. **API**: Consider adding signature verification API endpoint

## Troubleshooting

### "noblePostQuantum is not defined" error
- Verify `noble-post-quantum.min.js` is included in HTML
- Check file path is correct: `app/lib/noble-post-quantum.min.js`
- Ensure script loads before module scripts (app/main.js)
- Open browser console and check `window.noblePostQuantum`

### "Signature generation failed"
- Check browser console for details
- Verify noble library is loaded
- Ensure crypto.getRandomValues is available

### Signatures don't verify
- Ensure you're using the same private key
- Check that signed data format matches exactly
- Verify no whitespace/encoding issues

## Resources

- Read `PQ-CRYPTO-README.md` for detailed documentation
- Check Noble library docs: https://github.com/paulmillr/noble-post-quantum
- NIST PQC info: https://csrc.nist.gov/projects/post-quantum-cryptography
