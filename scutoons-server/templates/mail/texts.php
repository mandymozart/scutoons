<?php
// Texts of the login mails (templates/mail/login.*, no-account.*). Scutoons voice: short, literal, informal.
// Placeholders: {otp}, {code}, {minutes}.
return [
    'en' => [
        'subject' => '{otp} is your SCUTOONS code',
        'intro' => 'Confirm your email address to sign in to SCUTOONS.',
        'claim' => 'After signing in, the code {code} is added to your inventory.',
        'button' => 'Sign in',
        'otp' => 'Or enter this code:',
        'validity' => 'The link and the code work for {minutes} minutes.',
        'ignore' => 'You did not ask for this? Just ignore this email – nothing happens without your confirmation.',
        'signup' => 'This creates your SCUTOONS account.',
        'na_subject' => 'Your SCUTOONS sign-in',
        'na_intro' => 'Someone (hopefully you) wanted to sign in to SCUTOONS with this email address. There is no account for it yet.',
        'na_code' => 'The code {code} cannot be used to create an account (it is not valid, expired or already in someone\'s account).',
        'na_how' => 'To create an account you need a code from a SCUTOONS product (the QR code of a limited edition) or a promo code. Scan it or open it, then choose "Create account" or "Add to my account".',
        'na_read' => 'Reading works without an account: scan your code and the chapters unlock on your device.',
        'na_button' => 'Open SCUTOONS',
        'na_ignore' => 'You did not ask for this? Just ignore this email – nothing was created.',
    ],
    'de' => [
        'subject' => '{otp} ist dein SCUTOONS-Code',
        'intro' => 'Bestätige deine E-Mail-Adresse, um dich bei SCUTOONS anzumelden.',
        'claim' => 'Nach der Anmeldung wird der Code {code} deinem Inventar hinzugefügt.',
        'button' => 'Anmelden',
        'otp' => 'Oder gib diesen Code ein:',
        'validity' => 'Link und Code gelten {minutes} Minuten.',
        'ignore' => 'Du hast das nicht angefordert? Dann ignoriere diese E-Mail einfach – ohne deine Bestätigung passiert nichts.',
        'signup' => 'Damit wird dein SCUTOONS-Konto erstellt.',
        'na_subject' => 'Deine Anmeldung bei SCUTOONS',
        'na_intro' => 'Jemand (hoffentlich du) wollte sich mit dieser E-Mail-Adresse bei SCUTOONS anmelden. Dafür gibt es noch kein Konto.',
        'na_code' => 'Mit dem Code {code} kann kein Konto erstellt werden (ungültig, abgelaufen oder schon in einem anderen Konto).',
        'na_how' => 'Für ein Konto brauchst du einen Code von einem SCUTOONS-Produkt (der QR-Code einer limitierten Edition) oder einen Promo-Code. Scanne oder öffne ihn und wähle dann „Konto erstellen“ oder „Zu meinem Konto hinzufügen“.',
        'na_read' => 'Lesen geht auch ohne Konto: Scanne deinen Code, dann sind die Kapitel auf deinem Gerät freigeschaltet.',
        'na_button' => 'SCUTOONS öffnen',
        'na_ignore' => 'Du hast das nicht angefordert? Dann ignoriere diese E-Mail einfach – es wurde nichts angelegt.',
    ],
];
