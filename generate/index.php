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
    <script src="https://cdnjs.cloudflare.com/ajax/libs/qrcodejs/1.0.0/qrcode.min.js"></script>
    <style>
        #qrResult {
            margin-top: 30px;
            text-align: center;
            padding: 20px;
            background-color: #f8f9fa;
            border-radius: 5px;
            display: none;
        }
        #qrCode {
            margin: 20px 0;
        }
        .qr-info {
            background-color: #e9ecef;
            padding: 15px;
            border-radius: 5px;
            font-family: monospace;
            word-break: break-all;
            margin: 10px 0;
            text-align: left;
        }
        .error {
            color: #dc3545;
            margin: 10px 0;
        }
        .success {
            color: #28a745;
            margin: 10px 0;
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
                    
                    // Generate QR code using QRCode.js
                    new QRCode(document.getElementById('qrCode'), {
                        text: qrUrl,
                        width: 256,
                        height: 256,
                        colorDark: '#000000',
                        colorLight: '#FFFFFF',
                        correctLevel: QRCode.CorrectLevel.H,

                    });
                    
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