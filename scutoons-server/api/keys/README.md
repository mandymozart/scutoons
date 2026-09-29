# Post-Quantum Cryptographic Keys

This directory stores the ML-DSA-65 keypair used for signing QR code certificates.

## 🚀 Initial Setup

### Step 1: Generate Keys (ONE TIME ONLY)

Visit in your browser:
```
https://qr.scutoons.com/api/keys/generate-keys.php
```

This will:
1. Generate a secure ML-DSA-65 keypair
2. Save it to `keypair.json`
3. Display the public key for your records

**⚠️ Run this ONLY ONCE!** Regenerating keys will invalidate all existing signatures.

### Step 2: Verify Keys Were Created

Check that this file exists:
```
/api/keys/keypair.json
```

Should contain:
```json
{
    "privateKey": "hex_string...",
    "publicKey": "hex_string...",
    "algorithm": "ML-DSA-65",
    "generated": "2025-10-19T00:00:00.000Z"
}
```

### Step 3: Secure the Files

**Production Security:**

1. **Restrict access to keypair.json:**
   ```bash
   chmod 600 keypair.json  # Owner read/write only
   ```

2. **Delete or restrict generate-keys.php:**
   ```bash
   # After generating keys, remove public access:
   chmod 000 generate-keys.php
   # Or delete it entirely
   rm generate-keys.php save-keys.php
   ```

3. **Add to .gitignore:**
   ```
   api/keys/keypair.json
   api/keys/*.key
   ```

4. **Backup the keys securely!**
   - Keep encrypted backup offline
   - Store in password manager
   - Use hardware security module (HSM) for production

## 🔐 File Structure

```
api/keys/
├── README.md              ← You are here
├── generate-keys.php      ← One-time key generation (delete after use!)
├── save-keys.php          ← Called by generate-keys.php (delete after use!)
├── keys.php               ← API endpoint to retrieve keys
└── keypair.json           ← PRIVATE KEY STORAGE (secure this!)
```

## 🔑 What Gets Loaded Where

### Server-Side (api/keys/)
- `keypair.json` - Contains both private and public keys
- Only accessible via PHP scripts
- **Private key never leaves the server** (in ideal setup)

### Client-Side (generate/)
- Keys loaded via `fetch('/api/keys/keys.php')`  
- Used to sign certificates in browser
- ⚠️ **Current limitation**: Private key exposed to client
  - Acceptable for trusted admin interface
  - **Not recommended** for public-facing apps
  - Future: Move signing to server-side

## 🛡️ Security Considerations

### Current Implementation

**Pros:**
- ✅ Keys stored on server, not in JavaScript code
- ✅ Same keys used across sessions
- ✅ Signatures remain verifiable
- ✅ Keys can be backed up and restored

**Cons:**
- ⚠️ Private key exposed to browser (via HTTPS)
- ⚠️ Anyone with admin access can extract keys
- ⚠️ Keys visible in browser dev tools

**Acceptable for:**
- Internal admin tools
- Trusted users only
- Development/testing

**NOT acceptable for:**
- Public websites
- Untrusted users
- High-security requirements

### Better Approach: Server-Side Signing

For production, implement server-side signing:

```
Client Request (QR data) 
    ↓
POST /api/sign.php
    ↓
Server: Load private key
    ↓
Server: Sign data with ML-DSA-65
    ↓
Return: { signature, publicKey }
    ↓
Client: Embed signature in PDF
```

This keeps the private key 100% server-side.

## 📋 Key Specifications

**Algorithm**: ML-DSA-65 (FIPS 204 / Dilithium)  
**Security Level**: NIST Level 3 (192-bit security)  
**Key Sizes**:
- Private Key: 4,032 bytes (8,064 hex chars)
- Public Key: 1,952 bytes (3,904 hex chars)  
**Signature Size**: 3,293 bytes (6,586 hex chars)

## 🔄 Key Rotation

If you need to regenerate keys:

1. **Backup old keys:**
   ```bash
   cp keypair.json keypair.json.backup.$(date +%Y%m%d)
   ```

2. **Delete current keys:**
   ```bash
   rm keypair.json
   ```

3. **Generate new keys:**
   Visit `/api/keys/generate-keys.php` again

4. **Update public key distribution:**
   - Notify users of key change
   - Publish new public key
   - Keep old keys for verifying old signatures

⚠️ **WARNING**: Old signatures will be invalid with new keys!

## 🆘 Troubleshooting

### "Keys not initialized" error

**Problem**: `keypair.json` doesn't exist  
**Solution**: Visit `/api/keys/generate-keys.php` to generate keys

### "Failed to load keys" error

**Problem**: `keys.php` can't read `keypair.json`  
**Solution**: Check file permissions and path

### "Invalid key length" error

**Problem**: Corrupted key file  
**Solution**: Delete and regenerate keys

### Keys work but signatures don't verify

**Problem**: Using different keys than when signature was created  
**Solution**: Restore correct `keypair.json` from backup

## 📞 Support

For issues:
1. Check browser console for detailed error messages
2. Verify `keypair.json` exists and is valid JSON
3. Check file permissions (should be readable by web server)
4. Ensure noble-post-quantum library is loaded

## 🔗 Related Documentation

- `KEY-MANAGEMENT.md` - Key concepts and storage strategy
- `SIGNATURE-STORAGE.md` - How signatures are stored and verified
- `PQ-CRYPTO-README.md` - Post-quantum cryptography details
