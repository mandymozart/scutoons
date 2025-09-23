<?php
require_once '../config.php';
?>
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SCUTOONS - QR</title>
    <script type="module">
        import { nanoid } from 'https://cdn.jsdelivr.net/npm/nanoid/nanoid.js';
        window.nanoid = nanoid;
    </script>
    <script src="lib/qrcode/qrcode.min.js"></script>
    <link rel="stylesheet" type="text/css" href="https://www.scutoons.com/assets/styles.css">
    <style>
        #qrResult {
            margin-top: 2rem;
            text-align: center;
            padding: 2rem;
            background-color: #f8f9fa;
            display: none;
        }
        #qrCode {
            margin: 2rem 0;
        }
        .qr-info {
            padding: 1rem;
            word-break: break-all;
            margin: 1rem 0;
            text-align: left;
        }
        .error {
            color: #dc3545;
            margin: 1rem 0;
        }
        .success {
            color: #28a745;
            margin: 1rem 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1></h1>
        <form id="qrForm">
            <div class="form-group">
                <label for="redirectUrl">Redirect URL:</label>
                <input type="url" id="redirectUrl" name="redirectUrl" required placeholder="https://example.com">
            </div>
            <div class="form-group">
                <label for="type">Type:</label>
                <select id="type" name="type">
                    <option value="event">Event</option>
                    <option value="membership">Membership</option>
                    <option value="xr">XR</option>
                    <option value="promo">Promo</option>
                </select>
            </div>
            <button type="submit">Generate QR Code</button>
        </form>
        
        <div id="qrResult">
            <h3>Your QR Code</h3>
            <div id="qrCode"></div>
            <div class="qr-info">
                <strong>QR URL:</strong><br>
                <span id="qrUrl"></span>
            </div>
            <div class="qr-info">
                <strong>Redirect to:</strong><br>
                <span id="redirectTo"></span>
            </div>
            <div class="qr-info">
                <strong>Code:</strong> <span id="code"></span>
            </div>
        </div>
        
        <div id="message"></div>
    </div>

    <script>
        document.getElementById('qrForm').addEventListener('submit', async function(e) {
            e.preventDefault();
            
            // Generate nanoid code  
            const code = window.nanoid(8);
            const messageDiv = document.getElementById('message');
            messageDiv.innerHTML = '';
            
            try {
                const formData = new FormData(this);
                formData.append('code', code);
                
                const response = await fetch('generate.php', {
                    method: 'POST',
                    body: formData
                });
                
                const result = await response.json();
                
                if (result.success) {
                    // Generate QR code
                    const qrUrl = `https://qr.scutoons.com?c=${result.code}`;
                    
                    // Clear previous QR code
                    document.getElementById('qrCode').innerHTML = '';
                    
                    // Generate SVG QR code
                    const qrSvg = new QRCode({
                        msg: qrUrl,
                        dim: 256,
                        pad: 4,
                        ecl: 'H',
                        pal: ['#000000', '#FFFFFF']
                    });
                    
                    document.getElementById('qrCode').appendChild(qrSvg);
                    
                    // Show result
                    document.getElementById('qrUrl').textContent = qrUrl;
                    document.getElementById('redirectTo').textContent = result.redirect_to;
                    document.getElementById('code').textContent = result.code;
                    document.getElementById('qrResult').style.display = 'block';
                    
                    messageDiv.innerHTML = '<div class="success">QR Code generated successfully!</div>';
                    
                } else {
                    messageDiv.innerHTML = '<div class="error">Error: ' + result.message + '</div>';
                }
            } catch (error) {
                messageDiv.innerHTML = '<div class="error">Error generating QR code: ' + error.message + '</div>';
            }
        });
    </script>
</body>
</html>