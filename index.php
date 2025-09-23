<?php
require_once 'config.php';

// Handle email claiming
if ($_SERVER['REQUEST_METHOD'] === 'POST' && isset($_POST['claim_code'])) {
    $code = $_POST['claim_code'];
    $email = $_POST['email'] ?? '';
    
    if (!empty($email) && filter_var($email, FILTER_VALIDATE_EMAIL)) {
        try {
            $pdo = getDbConnection();
            $stmt = $pdo->prepare("UPDATE qr SET email = ?, claimed_at = NOW() WHERE code = ?");
            $stmt->execute([$email, $code]);
            
            // Get the redirect URL and redirect
            $stmt = $pdo->prepare("SELECT redirect_to FROM qr WHERE code = ?");
            $stmt->execute([$code]);
            $result = $stmt->fetch();
            
            if ($result) {
                header("Location: " . $result['redirect_to']);
                exit();
            }
        } catch (Exception $e) {
            $error = "Database error: " . $e->getMessage();
        }
    } else {
        $error = "Please provide a valid email address.";
    }
}

// Check if 'c' parameter exists in URL
if (isset($_GET['c'])) {
    $code = $_GET['c'];
    
    try {
        $pdo = getDbConnection();
        
        // Query database for matching code
        $stmt = $pdo->prepare("SELECT * FROM qr WHERE code = ?");
        $stmt->execute([$code]);
        $result = $stmt->fetch();
        
        if ($result) {
            // Check if already claimed (has email)
            if (!empty($result['email'])) {
                // Already claimed, redirect immediately
                header("Location: " . $result['redirect_to']);
                exit();
            } else {
                // Not claimed yet, show email form
                ?>
                <!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="UTF-8">
                    <meta name="viewport" content="width=device-width, initial-scale=1.0">
                    <title>Claim QR Code</title>
                    <style>
                        body {
                            font-family: Arial, sans-serif;
                            max-width: 500px;
                            margin: 50px auto;
                            padding: 20px;
                            background-color: #f5f5f5;
                        }
                        .container {
                            background: white;
                            padding: 30px;
                            border-radius: 10px;
                            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                            text-align: center;
                        }
                        h1 {
                            color: #333;
                            margin-bottom: 20px;
                        }
                        .info {
                            background-color: #e3f2fd;
                            padding: 15px;
                            border-radius: 5px;
                            margin: 20px 0;
                            color: #1976d2;
                        }
                        input[type="email"] {
                            width: 100%;
                            padding: 12px;
                            border: 1px solid #ddd;
                            border-radius: 5px;
                            font-size: 16px;
                            margin: 10px 0;
                            box-sizing: border-box;
                        }
                        button {
                            background-color: #007bff;
                            color: white;
                            padding: 12px 30px;
                            border: none;
                            border-radius: 5px;
                            font-size: 16px;
                            cursor: pointer;
                            margin: 5px;
                        }
                        button:hover {
                            background-color: #0056b3;
                        }
                        .skip-btn {
                            background-color: #6c757d;
                        }
                        .skip-btn:hover {
                            background-color: #545b62;
                        }
                        .error {
                            color: #dc3545;
                            margin: 10px 0;
                        }
                    </style>
                </head>
                <body>
                    <div class="container">
                        <h1>QR Code Found!</h1>
                        <div class="info">
                            This QR code will redirect you to:<br>
                            <strong><?php echo htmlspecialchars($result['redirect_to']); ?></strong>
                        </div>
                        
                        <?php if (isset($error)): ?>
                            <div class="error"><?php echo htmlspecialchars($error); ?></div>
                        <?php endif; ?>
                        
                        <p>Would you like to provide your email to claim this QR code?</p>
                        
                        <form method="POST">
                            <input type="hidden" name="claim_code" value="<?php echo htmlspecialchars($code); ?>">
                            <input type="email" name="email" placeholder="your@email.com" required>
                            <br>
                            <button type="submit">Claim & Continue</button>
                        </form>
                        
                        <form method="GET" style="display: inline;">
                            <input type="hidden" name="c" value="<?php echo htmlspecialchars($code); ?>">
                            <input type="hidden" name="skip" value="1">
                            <button type="submit" class="skip-btn">Skip & Continue</button>
                        </form>
                    </div>
                </body>
                </html>
                <?php
                exit();
            }
        } else {
            // Code not found
            http_response_code(404);
            echo "QR Code not found";
            exit();
        }
    } catch (Exception $e) {
        http_response_code(500);
        echo "Error: " . $e->getMessage();
        exit();
    }
} elseif (isset($_GET['skip']) && isset($_GET['c'])) {
    // Skip email, just redirect
    $code = $_GET['c'];
    try {
        $pdo = getDbConnection();
        $stmt = $pdo->prepare("SELECT redirect_to FROM qr WHERE code = ?");
        $stmt->execute([$code]);
        $result = $stmt->fetch();
        
        if ($result) {
            header("Location: " . $result['redirect_to']);
            exit();
        }
    } catch (Exception $e) {
        http_response_code(500);
        echo "Error: " . $e->getMessage();
        exit();
    }
} else {
    // No 'c' parameter, show simple landing page
    ?>
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>QR Code System</title>
        <style>
            body {
                font-family: Arial, sans-serif;
                max-width: 600px;
                margin: 100px auto;
                padding: 20px;
                background-color: #f5f5f5;
                text-align: center;
            }
            .container {
                background: white;
                padding: 40px;
                border-radius: 10px;
                box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            }
            h1 {
                color: #333;
                margin-bottom: 20px;
            }
            p {
                color: #666;
                font-size: 18px;
                line-height: 1.6;
            }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>Welcome to SCUTOONS</h1>
            <p>Scan a QR code to get started. or type it manually here:</p>
            <form method="GET">
                <input type="text" name="c" placeholder="QR Code" required>
                <button type="submit">Scan</button>
            </form>
        </div>
    </body>
    </html>
    <?php
}
?>
