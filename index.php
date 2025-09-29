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
                $redirectUrl = $result['redirect_to'];
                // Add the QR code as a parameter to the redirect URL
                $separator = (strpos($redirectUrl, '?') !== false) ? '&' : '?';
                header("Location: " . $redirectUrl . $separator . "qr=" . urlencode($code));
                exit();
            }
        } catch (Exception $e) {
            $error = "Database error: " . $e->getMessage();
        }
    } else {
        $error = "Please provide a valid email address.";
    }
}

// Check if skip button was clicked (check this FIRST before just checking for 'c')
if (isset($_GET['skip']) && isset($_GET['c'])) {
    // Skip email, just redirect
    $code = $_GET['c'];
    try {
        $pdo = getDbConnection();
        $stmt = $pdo->prepare("SELECT redirect_to FROM qr WHERE code = ?");
        $stmt->execute([$code]);
        $result = $stmt->fetch();

        if ($result) {
            $redirectUrl = $result['redirect_to'];
            // Add the QR code as a parameter to the redirect URL
            $separator = (strpos($redirectUrl, '?') !== false) ? '&' : '?';
            header("Location: " . $redirectUrl . $separator . "code=" . urlencode($code));
            exit();
        } else {
            // Code not found
            http_response_code(404);
            echo "QR Code not found. Please contact us!";
            exit();
        }
    } catch (Exception $e) {
        http_response_code(500);
        echo "Error: " . $e->getMessage();
        exit();
    }
}
// Check if 'c' parameter exists in URL (without skip)
elseif (isset($_GET['c'])) {
    $code = $_GET['c'];

    try {
        $pdo = getDbConnection();

        // Query database for matching code
        $stmt = $pdo->prepare("SELECT * FROM qr WHERE code = ?");
        $stmt->execute([$code]);
        $result = $stmt->fetch();

        if ($result) {
            // Check if already claimed (has email) and if the type is event or xr and not member or promo
            if (!empty($result['email']) || $result['type'] === 'event' || $result['type'] === 'xr') {
                // Already claimed, redirect immediately with QR code parameter
                $redirectUrl = $result['redirect_to'];
                $separator = (strpos($redirectUrl, '?') !== false) ? '&' : '?';
                header("Location: " . $redirectUrl . $separator . "code=" . urlencode($code));
                exit();
            } else {
                // Not claimed yet, show email form
?>
                <!DOCTYPE html>
                <html lang="en">

                <head>
                    <meta charset="UTF-8">
                    <meta name="viewport" content="width=device-width, initial-scale=1.0">
                    <title>SCUTOONS / Claim QR Code</title>
                    <link rel="stylesheet" type="text/css" href="https://www.scutoons.com/assets/styles.css">
                    <style>
                        .container {
                            background: white;
                            padding: 30px;
                            border: 4px solid black;
                            text-align: center;
                            margin-bottom: 2rem;
                        }

                        .info {
                            padding: 1rem;
                            margin: 2rem 0;
                            color: #1976d2;
                        }

                        .skip-btn {
                            background-color: #6c757d;
                        }

                        .skip-btn:hover {
                            background-color: #545b62;
                        }

                        .error {
                            color: #dc3545;
                            margin: 1rem 0;
                        }
                    </style>
                </head>

                <body>
                    <div class="container">
                        <h1><?= $result['type'] ?> QR Code Found! <strong><?= $result['code'] ?></strong></h1>
                        <div class="info">
                            This QR code will redirect you to:<br>
                            <strong><?php echo htmlspecialchars($result['redirect_to']); ?></strong>
                        </div>

                        <?php if (isset($error)): ?>
                            <div class="error"><?php echo htmlspecialchars($error); ?></div>
                        <?php endif; ?>
                        <h3>Claim this QR code</h3>
                        <p>Would you like to receive updates and new editions to your email?  Not sure? Skip it! </p>

                        <form method="GET" style="display: inline;">
                            <input type="hidden" name="c" value="<?php echo htmlspecialchars($code); ?>">
                            <input type="hidden" name="skip" value="1">
                            <button type="submit" class="skip-btn">Skip</button>
                        </form>
                        
                        <form method="POST">
                            <input type="hidden" name="claim_code" value="<?php echo htmlspecialchars($code); ?>">
                            <input type="email" name="email" placeholder="your@email.com" required>
                            <br>
                            <button type="submit">Claim</button>
                        </form>

                    </div>
                    <div class="logo">
                        <img src="https://www.scutoons.com/SCUTOONS-logo.svg" alt="SCUTOONS" />
                    </div>
                </body>

                </html>
    <?php
                exit();
            }
        } else {
            // Code not found
            http_response_code(404);
            echo "QR Code not found. Please contact us!";
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
        <title>SCUTOONS / Members Area</title>
        <link rel="stylesheet" type="text/css" href="https://www.scutoons.com/assets/styles.css">
        <style>
            .container {
                background: white;
                padding: 30px;
                border: 4px solid black;
                text-align: center;
                margin-bottom: 2rem;
            }
            
        </style>
    </head>

    <body>
        <div class="container">
            <h1>Welcome to our Members Area</h1>
            <p>Scan a QR code to get started or type it manually here:</p>
            <form method="GET">
                <input type="text" name="c" placeholder="Access Code" required>
                <button type="submit">Enter</button>
            </form>
        </div>
        <div class="logo">
            <img src="https://www.scutoons.com/SCUTOONS-logo.svg" alt="SCUTOONS" />
        </div>
    </body>

    </html>
<?php
}
?>