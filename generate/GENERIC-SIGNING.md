# Generic Signature Implementation

## Overview
The post-quantum signature system now signs **ALL fields** generically, not just core QR data. This ensures complete authenticity verification for any certificate data.

## What Gets Signed

**Everything in the QR object**, including:

### Core QR Fields
- `code` - The QR code
- `type` - Type (membership, event, xr, promo)
- `redirect_to` - Redirect URL
- `url` - The QR URL
- `created_at` - Creation timestamp
- `expires_at` - Expiration (if set)

### Template Fields (Dynamic)
- `receiver` - Recipient name
- `edition_number` - Edition number
- `edition_count` - Total editions
- `creators` - Artists/creators
- `productname` - Product name
- **Any future fields added to forms**

## Implementation

### Signing (crypto.js)
```javascript
export async function signQRCode(qrData) {
    const ml_dsa65 = getMlDsa();
    const keypair = await initializeKeypair();
    
    // Signs ENTIRE qrData object - completely generic
    const dataToSign = JSON.stringify(qrData);
    
    const message = new TextEncoder().encode(dataToSign);
    const signature = ml_dsa65.sign(message, keypair.secretKey);
    
    return {
        signature: bytesToHex(signature),
        publicKey: bytesToHex(keypair.publicKey),
        algorithm: 'ML-DSA-65',
        signedData: dataToSign,  // The exact JSON that was signed
        timestamp: new Date().toISOString()
    };
}
```

### Data Collection (generate.js)
```javascript
// Collect ALL template fields dynamically
this.templateData = {};
for (const [key, value] of formData.entries()) {
    if (!GenerateView.EXCLUDED_TEMPLATE_FIELDS.includes(key)) {
        this.templateData[key] = value || '';
    }
}

// Merge everything into QR object for signing
this.currentQR = {
    code: result.code,
    type: result.type,
    redirect_to: result.redirect_to,
    url: qrUrl,
    created_at: new Date().toISOString(),
    expires_at: result.expires_at || null,
    ...this.templateData,  // Spread ALL template fields
    svg: null
};

// Sign the complete object
const signatureData = await signQRCode(this.currentQR);
```

## Benefits

### 1. Future-Proof
- New form fields automatically signed
- No code changes needed for new fields
- Template system can evolve freely

### 2. Complete Authenticity
- **Certificate fields verified**: Receiver, edition, artists
- **Prevents tampering**: Any field modification breaks signature
- **Full audit trail**: All data cryptographically bound

### 3. Flexibility
- Different QR types can have different fields
- Template parser adds fields dynamically
- No hardcoded field lists

## Example Signed Data

```json
{
  "code": "ABC123",
  "type": "membership",
  "redirect_to": "https://scutoons.com/catpope",
  "url": "https://qr.scutoons.com?c=ABC123",
  "created_at": "2025-10-19T01:04:07.543Z",
  "expires_at": null,
  "receiver": "John Doe",
  "edition_number": "3",
  "edition_count": "12",
  "creators": "Mandy Mozart, Artist 2",
  "productname": "SCUTOONS Hoodie with QR Code"
}
```

**All of this is cryptographically signed and verified!**

## Security Guarantees

✅ **Cannot modify receiver name** - breaks signature
✅ **Cannot change edition number** - breaks signature  
✅ **Cannot alter artist credits** - breaks signature
✅ **Cannot forge certificates** - requires private key
✅ **Quantum-resistant** - ML-DSA-65 (FIPS 204)

## Adding New Fields

To add new fields that will be automatically signed:

1. Add field to template (e.g., `{{warranty_years}}`)
2. Field appears in form automatically
3. User fills it in
4. **Automatically included in signature** ✨
5. Verification checks the field

No code changes needed!

## Important Notes

- `svg` field is excluded from signing (not relevant for verification)
- `code` is included but also used for file naming
- The exact JSON string is what's signed (order, formatting matter)
- Field names are case-sensitive
- Empty values are stored as empty strings `""`
