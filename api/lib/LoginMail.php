<?php
declare(strict_types=1);

/**
 * The login email (port of osct LoginMail): a magic link and a 6-digit one-time code (typed into the page/app –
 * needed where the link opens another browser). Mentions the code that will be added to the inventory.
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
        ],
        'de' => [
            'subject' => '{otp} ist dein SCUTOONS-Code',
            'intro' => 'Bestätige deine E-Mail-Adresse, um dich bei SCUTOONS anzumelden.',
            'claim' => 'Nach der Anmeldung wird der Code {code} deinem Inventar hinzugefügt.',
            'button' => 'Anmelden',
            'otp' => 'Oder gib diesen Code ein:',
            'validity' => 'Link und Code gelten {minutes} Minuten.',
            'ignore' => 'Du hast das nicht angefordert? Dann ignoriere diese E-Mail einfach – ohne deine Bestätigung passiert nichts.',
        ],
    ];

    public static function locale(mixed $value): string
    {
        if (!is_string($value)) return 'en';
        $locale = strtolower(substr($value, 0, 2));
        return isset(self::TEXTS[$locale]) ? $locale : 'en';
    }

    /** @return array{subject: string, text: string, html: string} */
    public static function compose(string $locale, string $link, string $otp, ?string $claimCode): array
    {
        $texts = self::TEXTS[$locale] ?? self::TEXTS['en'];
        $vars = ['{otp}' => $otp, '{code}' => (string) $claimCode, '{minutes}' => (string) Config::int('LOGIN_TTL_MINUTES')];
        $t = fn(string $key) => strtr($texts[$key], $vars);
        $e = fn(string $value) => htmlspecialchars($value, ENT_QUOTES, 'UTF-8');

        $text = $t('intro') . "\n\n";
        if ($claimCode !== null) $text .= $t('claim') . "\n\n";
        $text .= $link . "\n\n" . $t('otp') . "\n\n    " . $otp . "\n\n" . $t('validity') . "\n" . $t('ignore') . "\n\n– SCUTOONS\n";

        $mono = "'IBM Plex Mono',Menlo,Consolas,monospace";
        $html = '<!doctype html><html lang="' . $e($locale) . '"><body style="margin:0;padding:24px;background:#ffffff;color:#000000;font-family:' . $mono . ';font-size:15px;line-height:1.4">'
            . '<div style="max-width:480px;margin:0 auto;border:2px solid #000000;padding:24px">'
            . '<p style="margin:0 0 20px;font-weight:700;letter-spacing:1px">SCUTOONS</p>'
            . '<p style="margin:0 0 20px">' . $e($t('intro')) . '</p>'
            . ($claimCode !== null ? '<p style="margin:0 0 20px">' . $e($t('claim')) . '</p>' : '')
            . '<p style="margin:0 0 24px"><a href="' . $e($link) . '" style="display:inline-block;padding:12px 32px;background:#000000;color:#ffffff;text-decoration:none">' . $e($t('button')) . '</a></p>'
            . '<p style="margin:0 0 8px">' . $e($t('otp')) . '</p>'
            . '<p style="margin:0;font-size:28px;letter-spacing:6px;font-weight:700">' . $e($otp) . '</p>'
            . '<p style="margin:24px 0 0;color:#6b6b6b;font-size:12px">' . $e($t('validity')) . '<br>' . $e($t('ignore')) . '</p>'
            . '</div></body></html>';

        return ['subject' => $t('subject'), 'text' => $text, 'html' => $html];
    }
}
