<?php
declare(strict_types=1);

/**
 * The login email (port of osct LoginMail): a magic link and a 6-digit one-time code (typed into the page/app –
 * needed where the link opens another browser). Mentions the code that will be added to the inventory.
 * composeNoAccount(): the mail for an address without an account that asked without a signup code – no link, no
 * one-time code, just how to get an account (a code from a SCUTOONS product or a promo code).
 * Scutoons voice: short, literal, informal. Design: ink on paper, monospace, square 2px button.
 */
final class LoginMail
{
    private const TEXTS = [
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

    public static function locale(mixed $value): string
    {
        if (!is_string($value)) return 'en';
        $locale = strtolower(substr($value, 0, 2));
        return isset(self::TEXTS[$locale]) ? $locale : 'en';
    }

    /**
     * $claimCode = a claimable code that is added to the inventory; $signup = confirming creates the account
     * @return array{subject: string, text: string, html: string}
     */
    public static function compose(string $locale, string $link, string $otp, ?string $claimCode, bool $signup = false): array
    {
        $texts = self::TEXTS[$locale] ?? self::TEXTS['en'];
        $vars = ['{otp}' => $otp, '{code}' => (string) $claimCode, '{minutes}' => (string) Config::int('LOGIN_TTL_MINUTES')];
        $t = fn(string $key) => strtr($texts[$key], $vars);
        $e = fn(string $value) => htmlspecialchars($value, ENT_QUOTES, 'UTF-8');

        $text = $t('intro') . "\n\n";
        if ($signup) $text .= $t('signup') . "\n\n";
        if ($claimCode !== null) $text .= $t('claim') . "\n\n";
        $text .= $link . "\n\n" . $t('otp') . "\n\n    " . $otp . "\n\n" . $t('validity') . "\n" . $t('ignore') . "\n\n– SCUTOONS\n";

        $mono = "'IBM Plex Mono',Menlo,Consolas,monospace";
        $html = '<!doctype html><html lang="' . $e($locale) . '"><body style="margin:0;padding:24px;background:#ffffff;color:#000000;font-family:' . $mono . ';font-size:15px;line-height:1.4">'
            . '<div style="max-width:480px;margin:0 auto;border:2px solid #000000;padding:24px">'
            . '<p style="margin:0 0 20px;font-weight:700;letter-spacing:1px">SCUTOONS</p>'
            . '<p style="margin:0 0 20px">' . $e($t('intro')) . '</p>'
            . ($signup ? '<p style="margin:0 0 20px">' . $e($t('signup')) . '</p>' : '')
            . ($claimCode !== null ? '<p style="margin:0 0 20px">' . $e($t('claim')) . '</p>' : '')
            . '<p style="margin:0 0 24px"><a href="' . $e($link) . '" style="display:inline-block;padding:12px 32px;background:#000000;color:#ffffff;text-decoration:none">' . $e($t('button')) . '</a></p>'
            . '<p style="margin:0 0 8px">' . $e($t('otp')) . '</p>'
            . '<p style="margin:0;font-size:28px;letter-spacing:6px;font-weight:700">' . $e($otp) . '</p>'
            . '<p style="margin:24px 0 0;color:#6b6b6b;font-size:12px">' . $e($t('validity')) . '<br>' . $e($t('ignore')) . '</p>'
            . '</div></body></html>';

        return ['subject' => $t('subject'), 'text' => $text, 'html' => $html];
    }

    /**
     * No account for this address and no usable signup code: explains how to get an account. No link to sign in,
     * no one-time code. $code = the code sent with the request (not usable for a signup), if any.
     * @return array{subject: string, text: string, html: string}
     */
    public static function composeNoAccount(string $locale, string $viewerUrl, ?string $code): array
    {
        $texts = self::TEXTS[$locale] ?? self::TEXTS['en'];
        $t = fn(string $key) => strtr($texts[$key], ['{code}' => (string) $code]);
        $e = fn(string $value) => htmlspecialchars($value, ENT_QUOTES, 'UTF-8');
        $paragraphs = array_filter([$t('na_intro'), $code !== null ? $t('na_code') : null, $t('na_how'), $t('na_read')]);

        $text = implode("\n\n", $paragraphs) . "\n\n" . $viewerUrl . "\n\n" . $t('na_ignore') . "\n\n– SCUTOONS\n";

        $mono = "'IBM Plex Mono',Menlo,Consolas,monospace";
        $html = '<!doctype html><html lang="' . $e($locale) . '"><body style="margin:0;padding:24px;background:#ffffff;color:#000000;font-family:' . $mono . ';font-size:15px;line-height:1.4">'
            . '<div style="max-width:480px;margin:0 auto;border:2px solid #000000;padding:24px">'
            . '<p style="margin:0 0 20px;font-weight:700;letter-spacing:1px">SCUTOONS</p>'
            . implode('', array_map(fn($p) => '<p style="margin:0 0 20px">' . $e($p) . '</p>', $paragraphs))
            . '<p style="margin:0 0 24px"><a href="' . $e($viewerUrl) . '" style="display:inline-block;padding:12px 32px;background:#000000;color:#ffffff;text-decoration:none">' . $e($t('na_button')) . '</a></p>'
            . '<p style="margin:24px 0 0;color:#6b6b6b;font-size:12px">' . $e($t('na_ignore')) . '</p>'
            . '</div></body></html>';

        return ['subject' => $t('na_subject'), 'text' => $text, 'html' => $html];
    }
}
