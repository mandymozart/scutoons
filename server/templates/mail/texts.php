<?php
// Every text of every email, per locale → template id → key (the email module, agents/PLATFORM.md "Emails").
// The structure of each template (which keys, in which order, buttons, codes) is in src/Domain/Mail/MailTemplates.php;
// the HTML frame in layout.html.php. Scutoons voice: short, literal, informal.
// Placeholders: {name} = the template's variable `name` (MailTemplates "vars"), e.g. {otp}, {code}, {minutes}.
// `subject` is required per template; `_layout` holds the texts every mail shares.
return [
    'en' => [
        '_layout' => [
            'signature' => '– SCUTOONS',
        ],
        'login_code' => [
            'subject' => '{otp} is your SCUTOONS code',
            'intro' => 'Confirm your email address to sign in to SCUTOONS.',
            'signup' => 'This creates your SCUTOONS account.',
            'claim' => 'After signing in, the code {code} is added to your inventory.',
            'button' => 'Sign in',
            'otp' => 'Or enter this code:',
            'validity' => 'The link and the code work for {minutes} minutes.',
            'ignore' => 'You did not ask for this? Just ignore this email – nothing happens without your confirmation.',
        ],
        'signup_without_code' => [
            'subject' => 'Your SCUTOONS sign-in',
            'intro' => 'Someone (hopefully you) wanted to sign in to SCUTOONS with this email address. There is no account for it yet.',
            'code' => 'The code {code} cannot be used to create an account (it is not valid, expired or already in someone\'s account).',
            'how' => 'To create an account you need a code from a SCUTOONS product (the QR code of a limited edition) or a promo code. Scan it or open it, then choose "Create account" or "Add to my account".',
            'read' => 'Reading works without an account: scan your code and the chapters unlock on your device.',
            'button' => 'Open SCUTOONS',
            'ignore' => 'You did not ask for this? Just ignore this email – nothing was created.',
        ],
    ],
    'de' => [
        '_layout' => [
            'signature' => '– SCUTOONS',
        ],
        'login_code' => [
            'subject' => '{otp} ist dein SCUTOONS-Code',
            'intro' => 'Bestätige deine E-Mail-Adresse, um dich bei SCUTOONS anzumelden.',
            'signup' => 'Damit wird dein SCUTOONS-Konto erstellt.',
            'claim' => 'Nach der Anmeldung wird der Code {code} deinem Inventar hinzugefügt.',
            'button' => 'Anmelden',
            'otp' => 'Oder gib diesen Code ein:',
            'validity' => 'Link und Code gelten {minutes} Minuten.',
            'ignore' => 'Du hast das nicht angefordert? Dann ignoriere diese E-Mail einfach – ohne deine Bestätigung passiert nichts.',
        ],
        'signup_without_code' => [
            'subject' => 'Deine Anmeldung bei SCUTOONS',
            'intro' => 'Jemand (hoffentlich du) wollte sich mit dieser E-Mail-Adresse bei SCUTOONS anmelden. Dafür gibt es noch kein Konto.',
            'code' => 'Mit dem Code {code} kann kein Konto erstellt werden (ungültig, abgelaufen oder schon in einem anderen Konto).',
            'how' => 'Für ein Konto brauchst du einen Code von einem SCUTOONS-Produkt (der QR-Code einer limitierten Edition) oder einen Promo-Code. Scanne oder öffne ihn und wähle dann „Konto erstellen“ oder „Zu meinem Konto hinzufügen“.',
            'read' => 'Lesen geht auch ohne Konto: Scanne deinen Code, dann sind die Kapitel auf deinem Gerät freigeschaltet.',
            'button' => 'SCUTOONS öffnen',
            'ignore' => 'Du hast das nicht angefordert? Dann ignoriere diese E-Mail einfach – es wurde nichts angelegt.',
        ],
    ],
];
