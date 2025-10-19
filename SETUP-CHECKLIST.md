# 🚀 Post-Quantum QR System Setup Checklist

## ✅ Step-by-Step Setup

### 1. Generate Cryptographic Keys (ONE TIME)

Visit this URL in your browser:
```
http://localhost/api/keys/generate-keys.php
```
Or:
```
https://qr.scutoons.com/api/keys/generate-keys.php
```

**What happens:**
- Generates ML-DSA-65 keypair
- Saves to `/api/keys/keypair.json`
- Shows confirmation screen

**You only do this ONCE!**

---

### 2. Verify Key Files Created

Check these files exist:

```
✅ /api/keys/keypair.json          ← Private & public keys
✅ /generate/files/signatures/     ← Folder for signature storage (auto-created)
```

---

### 3. Test the System

#### A. Generate a QR Code

1. Go to: `http://localhost/generate/`
2. Fill in the form
3. Click "Generate"
4. Check browser console for:
   ```
   🔐 Loading keypair from server...
   ✅ Keypair loaded successfully
   ═══════════════════════════════════════
   POST-QUANTUM SIGNATURE GENERATED
   ═══════════════════════════════════════
   QR Code: yB56cs59
   ```

#### B. Check Files Created

```
/generate/files/
├── svgs/
│   └── SCUTOONS-QR-MEMBERSHIP-yB56cs59.svg    ✅
├── certificates/
│   └── SCUTOONS-COA-yB56cs59.pdf              ✅
└── signatures/
    └── SIGNATURE-yB56cs59.json                 ✅ NEW!
```

#### C. Verify the Signature

1. Go to: `http://localhost/generate/verify.html`
2. Enter the QR code: `yB56cs59`
3. Click "Verify by Code"
4. Should see: ✅ **Signature Valid!**

---

### 4. Secure the Keys (Production)

After generating keys:

```bash
# Restrict key file permissions
chmod 600 api/keys/keypair.json

# Optional: Delete generation scripts (can't regenerate keys!)
rm api/keys/generate-keys.php
rm api/keys/save-keys.php

# Backup the keys!
cp api/keys/keypair.json ~/secure-backup/keypair-$(date +%Y%m%d).json
```

Add to `.gitignore`:
```
api/keys/keypair.json
api/keys/*.backup
```

---

## 🗂️ File Locations

### Created by Setup

| File | Purpose | Public? |
|------|---------|---------|
| `/api/keys/keypair.json` | Private & public keys | ❌ NO - Keep secret! |
| `/api/keys/generate-keys.php` | Key generation UI | ⚠️ Delete after use |
| `/api/keys/save-keys.php` | Saves generated keys | ⚠️ Delete after use |
| `/api/keys/keys.php` | API to load keys | ⚠️ Restrict access |

### Created per QR Code

| File | Purpose | Public? |
|------|---------|---------|
| `/generate/files/signatures/SIGNATURE-{CODE}.json` | Signature data | ✅ YES - For verification |
| `/generate/files/svgs/SCUTOONS-QR-{TYPE}-{CODE}.svg` | QR code image | ✅ YES |
| `/generate/files/certificates/SCUTOONS-COA-{CODE}.pdf` | Certificate PDF | ✅ YES |

---

## 🔍 Verification Flow

```
User has PDF
    ↓
Sees: "QR Code: yB56cs59"
    ↓
Goes to: verify.html
    ↓
Enters code: yB56cs59
    ↓
System fetches: /files/signatures/SIGNATURE-yB56cs59.json
    ↓
Verifies signature using public key
    ↓
✅ "Certificate is authentic!"
```

---

## 🐛 Troubleshooting

### Error: "Keys not initialized"

**Problem**: Keys haven't been generated yet

**Solution**: Visit `/api/keys/generate-keys.php` and generate them

---

### Error: "Failed to load keypair"

**Problem**: Can't access `keypair.json`

**Check:**
1. File exists: `/api/keys/keypair.json`
2. File permissions allow web server to read
3. Path is correct in `keys.php`

---

### Signatures don't verify

**Problem**: Using different keys than when generated

**Solutions:**
1. Check `keypair.json` hasn't been replaced
2. Restore from backup if keys were regenerated
3. Check console for specific error messages

---

### "noble-post-quantum library not loaded"

**Problem**: JavaScript library not found

**Solution**: 
1. Check `/generate/app/lib/noble-post-quantum.min.js` exists
2. Verify `<script src="app/lib/noble-post-quantum.min.js"></script>` in HTML
3. Clear browser cache

---

## 📚 Documentation

- **`/api/keys/README.md`** - Key management details
- **`/generate/KEY-MANAGEMENT.md`** - Private vs public key explanation
- **`/generate/SIGNATURE-STORAGE.md`** - Storage architecture
- **`/generate/PQ-CRYPTO-README.md`** - Cryptography technical docs
- **`/generate/SETUP-PQ.md`** - Quick setup guide

---

## ⚡ Quick Commands

```bash
# Check if keys exist
ls -la api/keys/keypair.json

# Check signature files
ls -la generate/files/signatures/

# View a signature file
cat generate/files/signatures/SIGNATURE-yB56cs59.json | jq

# Backup keys
cp api/keys/keypair.json ~/backup/keys-$(date +%Y%m%d).json

# Test key loading
curl http://localhost/api/keys/keys.php | jq
```

---

## ✨ You're All Set!

Once you've completed the checklist:

✅ Keys are generated and stored securely  
✅ QR codes are signed with post-quantum cryptography  
✅ Signatures can be verified online  
✅ System is production-ready (with security notes applied)

**Next Steps:**
1. Generate some test QR codes
2. Download and check the PDFs
3. Verify signatures work
4. Implement additional security measures for production
