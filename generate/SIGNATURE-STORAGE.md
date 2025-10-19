# Post-Quantum Signature Storage Strategy

## File Structure

```
generate/files/
├── svgs/              # QR code SVG files
│   └── SCUTOONS-QR-{TYPE}-{CODE}.svg
├── certificates/      # Certificate of Authenticity PDFs
│   └── SCUTOONS-COA-{CODE}.pdf
├── qrs/              # Printable QR PDFs
│   └── SCUTOONS-QR-PRINT-{CODE}.pdf
└── signatures/       # ⭐ Post-Quantum Signature Files
    └── SIGNATURE-{CODE}.json
```

## Signature File Format

**File**: `files/signatures/SIGNATURE-{CODE}.json`

```json
{
  "code": "yB56cs59",
  "type": "membership",
  "signature": {
    "signature": "hex-encoded ML-DSA-65 signature (6594 chars)",
    "publicKey": "hex-encoded public key (3904 chars)",
    "algorithm": "ML-DSA-65",
    "signedData": "{\"code\":\"yB56cs59\",\"type\":\"membership\",...}",
    "timestamp": "2025-10-19T00:15:23.456Z"
  },
  "fingerprint": "ABCD:1234:5678:...",
  "created_at": "2025-10-19T00:15:23.456Z"
}
```

## Storage Locations

### 1. Server-Side (Database/Files)

**Purpose**: Audit trail, public verification

**What to store:**
- ✅ Full signature JSON file → `files/signatures/SIGNATURE-{CODE}.json`
- ✅ Public key (can be shared publicly)
- ✅ Signature metadata (timestamp, algorithm, fingerprint)
- ✅ Signed data hash

**What NOT to store:**
- ❌ Private key (keep this secure, not in files folder!)

**Access**: Public read-only via HTTPS

### 2. User's Certificate (PDF)

**Purpose**: Portable proof of authenticity

**What's included:**
- ✅ QR code (unique identifier)
- ✅ Fingerprint (human-readable verification)
- ✅ Algorithm info (ML-DSA-65)
- ✅ Verification instructions
- ✅ Link to online verification

**User can:**
- Print it
- Store in wallet
- Share with others
- Verify anytime at verify.html

### 3. Optional: Downloadable Signature File

**Purpose**: Advanced users, offline verification

**Implementation:**
```javascript
// Add button to download signature file
const downloadSigBtn = document.createElement('button');
downloadSigBtn.textContent = 'Download Signature File';
downloadSigBtn.onclick = () => {
    const blob = new Blob([JSON.stringify(signatureData, null, 2)], 
        { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `SIGNATURE-${code}.json`;
    a.click();
};
```

## Verification Methods

### Method 1: Quick Verification (Recommended)

**User Flow:**
1. Open PDF certificate
2. Note the QR code (e.g., `yB56cs59`)
3. Visit `https://qr.scutoons.com/generate/verify.html`
4. Enter code → Click "Verify by Code"
5. System fetches signature from server
6. Instant verification ✅

**Pros:**
- Simple for users
- No copy/paste needed
- Always uses latest signature

### Method 2: Full Signature Verification

**User Flow:**
1. Copy base64 signature from console or file
2. Visit verification page
3. Paste signature → Click "Verify by Signature"
4. Offline-capable verification ✅

**Pros:**
- Works offline (if library is cached)
- User has complete signature data
- No server dependency

## Security Considerations

### Private Key Management

**CRITICAL**: The private key must be stored securely!

**Options:**
1. **Environment Variables** (development)
   ```bash
   PQ_PRIVATE_KEY=hex_string_here
   ```

2. **Encrypted File** (production)
   ```javascript
   // Load and decrypt private key
   const encryptedKey = fs.readFileSync('keys/private.key.enc');
   const privateKey = decrypt(encryptedKey, process.env.KEY_PASSWORD);
   ```

3. **Hardware Security Module (HSM)** (enterprise)
   - AWS CloudHSM
   - Azure Key Vault
   - YubiHSM

### Public Key Distribution

**Public key should be:**
- Accessible at a well-known URL: `https://qr.scutoons.com/keys/public.key`
- Included in signature files
- Displayed on verification page
- Version-controlled with signatures

### Signature File Security

**Signature files are public** - anyone can read them to verify certificates.

**They contain:**
- ✅ Public key (safe to share)
- ✅ Signature (proves authenticity)
- ✅ Signed data (the QR code info)

**They do NOT contain:**
- ❌ Private key
- ❌ Sensitive user data
- ❌ Payment info

## API Endpoints (Optional)

For programmatic access, consider adding:

```javascript
// GET /api/signature/{code}
// Returns signature JSON for verification

// POST /api/verify
// Body: { code: "yB56cs59" }
// Returns: { valid: true, details: {...} }
```

## User Instructions

Include in your PDFs:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🔐 CERTIFICATE VERIFICATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

To verify this certificate is authentic:

1. Visit: https://qr.scutoons.com/generate/verify.html
2. Enter your QR code: yB56cs59
3. Click "Verify by Code"

Your certificate is protected by post-quantum 
cryptography (ML-DSA-65/Dilithium), ensuring 
authenticity even against quantum computers.

Fingerprint: ABCD:1234:5678:9ABC:DEF0:1234:5678
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## File Size Estimates

- **Signature file**: ~8-12 KB (JSON)
- **Public key**: ~2 KB (1952 bytes)
- **Private key**: ~4 KB (4032 bytes)
- **Signature**: ~3.3 KB (3293 bytes)

## Backup Strategy

**Daily backups of:**
1. `files/signatures/` directory
2. Private key (encrypted, separate location)
3. Database records (if using DB)

**Retention:**
- Keep signature files indefinitely
- Users may need to verify years later
- Post-quantum signatures remain valid long-term
