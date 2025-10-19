<?php
/**
 * One-time Key Generation Script
 * Run this ONCE to generate the ML-DSA-65 keypair
 * 
 * Usage: Visit this page in browser or run via command line
 * After keys are generated, access is blocked for security
 */

$keysFile = __DIR__ . '/keypair.json';

// Check if keys already exist
if (file_exists($keysFile)) {
    header('Content-Type: text/html');
    ?>
    <!DOCTYPE html>
    <html>
    <head>
        <title>Keys Already Generated</title>
        <style>
            body { font-family: monospace; padding: 40px; max-width: 800px; margin: 0 auto; }
            .warning { background: #fff3cd; border: 2px solid #ffc107; padding: 20px; border-radius: 8px; }
            .success { background: #d4edda; border: 2px solid #28a745; padding: 20px; border-radius: 8px; }
            code { background: #f5f5f5; padding: 2px 6px; border-radius: 3px; }
            .danger { background: #f8d7da; border: 2px solid #dc3545; padding: 20px; border-radius: 8px; margin-top: 20px; }
        </style>
    </head>
    <body>
        <div class="warning">
            <h2>⚠️ Keys Already Exist</h2>
            <p>The keypair has already been generated.</p>
            <p><strong>Location:</strong> <code><?php echo $keysFile; ?></code></p>
        </div>
        
        <div class="danger">
            <h3>🔒 Security Note</h3>
            <p>If you need to regenerate keys:</p>
            <ol>
                <li>Backup the existing <code>keypair.json</code> file</li>
                <li>Delete <code>keypair.json</code></li>
                <li>Refresh this page</li>
            </ol>
            <p><strong>WARNING:</strong> Regenerating keys will invalidate all existing signatures!</p>
        </div>
    </body>
    </html>
    <?php
    exit;
}

// Generate keys using JavaScript (since noble-post-quantum is JS library)
?>
<!DOCTYPE html>
<html>
<head>
    <title>Generate ML-DSA-65 Keypair</title>
    <script src="../../generate/app/lib/noble-post-quantum.min.js"></script>
    <style>
        body { 
            font-family: monospace; 
            padding: 40px; 
            max-width: 1000px; 
            margin: 0 auto;
            background: #f5f5f5;
        }
        .container {
            background: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        button {
            background: #007bff;
            color: white;
            padding: 15px 30px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 16px;
            font-weight: bold;
        }
        button:hover { background: #0056b3; }
        button:disabled { background: #ccc; cursor: not-allowed; }
        .output {
            background: #f8f9fa;
            padding: 20px;
            border-radius: 4px;
            margin-top: 20px;
            border-left: 4px solid #007bff;
            display: none;
        }
        .output.show { display: block; }
        .success { 
            background: #d4edda; 
            border-left-color: #28a745;
            padding: 20px;
            border-radius: 4px;
            margin-top: 20px;
        }
        .warning {
            background: #fff3cd;
            border: 2px solid #ffc107;
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 20px;
        }
        code {
            background: #e9ecef;
            padding: 2px 6px;
            border-radius: 3px;
            word-break: break-all;
        }
        .key-display {
            max-height: 150px;
            overflow-y: auto;
            word-break: break-all;
            font-size: 11px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🔐 Generate Post-Quantum Keypair</h1>
        
        <div class="warning">
            <strong>⚠️ IMPORTANT:</strong> This will generate a new ML-DSA-65 keypair.
            <ul>
                <li>Run this ONLY ONCE</li>
                <li>Keys will be stored on the server</li>
                <li>All future QR codes will be signed with these keys</li>
                <li>If you regenerate, old signatures become invalid</li>
            </ul>
        </div>
        
        <button id="generateBtn" onclick="generateKeys()">Generate Keys Now</button>
        
        <div id="output" class="output">
            <h3>Generating...</h3>
            <p id="status">Initializing...</p>
        </div>
        
        <div id="result"></div>
    </div>
    
    <script>
        async function generateKeys() {
            const btn = document.getElementById('generateBtn');
            const output = document.getElementById('output');
            const status = document.getElementById('status');
            const result = document.getElementById('result');
            
            btn.disabled = true;
            output.classList.add('show');
            
            try {
                // Check if library is loaded
                if (!window.noblePostQuantum || !window.noblePostQuantum.ml_dsa65) {
                    throw new Error('noble-post-quantum library not loaded');
                }
                
                status.textContent = 'Generating random seed...';
                await new Promise(resolve => setTimeout(resolve, 100));
                
                const ml_dsa65 = window.noblePostQuantum.ml_dsa65;
                const seed = crypto.getRandomValues(new Uint8Array(32));
                
                status.textContent = 'Generating keypair (this may take a moment)...';
                await new Promise(resolve => setTimeout(resolve, 100));
                
                const keygenResult = ml_dsa65.keygen(seed);
                const secretKey = keygenResult.secretKey;
                const publicKey = keygenResult.publicKey;
                
                // Convert to hex
                const secretKeyHex = Array.from(secretKey).map(b => b.toString(16).padStart(2, '0')).join('');
                const publicKeyHex = Array.from(publicKey).map(b => b.toString(16).padStart(2, '0')).join('');
                
                status.textContent = 'Saving to server...';
                
                // Save to server
                const response = await fetch('save-keys.php', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        privateKey: secretKeyHex,
                        publicKey: publicKeyHex,
                        generated: new Date().toISOString()
                    })
                });
                
                const saveResult = await response.json();
                
                if (saveResult.success) {
                    result.innerHTML = `
                        <div class="success">
                            <h3>✅ Keys Generated Successfully!</h3>
                            <p><strong>Private Key Length:</strong> ${secretKey.length} bytes</p>
                            <p><strong>Public Key Length:</strong> ${publicKey.length} bytes</p>
                            <p><strong>Saved to:</strong> <code>${saveResult.file}</code></p>
                            <p><strong>Generated:</strong> ${new Date().toLocaleString()}</p>
                            
                            <h4>Public Key (hex):</h4>
                            <div class="key-display"><code>${publicKeyHex}</code></div>
                            
                            <h4 style="margin-top: 20px;">⚠️ Next Steps:</h4>
                            <ol>
                                <li>Keep <code>keypair.json</code> secure and backed up</li>
                                <li>The system will now use these keys for all signatures</li>
                                <li><strong>Delete or restrict access to this generation script</strong></li>
                            </ol>
                        </div>
                    `;
                    output.style.display = 'none';
                } else {
                    throw new Error(saveResult.error || 'Failed to save keys');
                }
                
            } catch (error) {
                result.innerHTML = `
                    <div style="background: #f8d7da; padding: 20px; border-radius: 4px; margin-top: 20px;">
                        <h3>❌ Error</h3>
                        <p>${error.message}</p>
                    </div>
                `;
                btn.disabled = false;
            }
        }
    </script>
</body>
</html>
<?php
exit;
?>
