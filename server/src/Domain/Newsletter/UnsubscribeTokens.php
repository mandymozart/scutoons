<?php
declare(strict_types=1);

namespace Scutoons\Domain\Newsletter;

use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\Secrets;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The unsubscribe links of the newsletter: `API_URL/api/newsletter/unsubscribe?t=<token>`, token =
 * base64url("<locale>|<email>") "." base64url(HMAC-SHA256 truncated to 16 bytes, own key label). Nothing stored; the
 * link never expires (an old mail must still unsubscribe), it only carries the address and the page's language.
 */
final class UnsubscribeTokens
{
    public function __construct(private readonly Secrets $secrets, private readonly Settings $settings)
    {
    }

    public function token(string $email, string $locale): string
    {
        $payload = MailTemplates::locale($locale) . '|' . strtolower(trim($email));
        return self::b64($payload) . '.' . self::b64($this->mac($payload));
    }

    public function url(string $email, string $locale): string
    {
        return rtrim($this->settings->get('API_URL'), '/') . '/api/newsletter/unsubscribe?t=' . $this->token($email, $locale);
    }

    /** @return array{email: string, locale: string}|null null = not a token of ours */
    public function read(mixed $token): ?array
    {
        if (!is_string($token) || strlen($token) > 800 || substr_count($token, '.') !== 1) return null;
        [$payload, $mac] = explode('.', $token);
        $payload = self::unb64($payload);
        $mac = self::unb64($mac);
        if ($payload === null || $mac === null || !hash_equals($this->mac($payload), $mac)) return null;
        $parts = explode('|', $payload, 2);
        if (count($parts) !== 2 || $parts[1] === '') return null;
        return ['email' => $parts[1], 'locale' => MailTemplates::locale($parts[0])];
    }

    private function mac(string $payload): string
    {
        return substr(hash_hmac('sha256', $payload, $this->secrets->derivedKey('newsletter-unsubscribe'), true), 0, 16);
    }

    private static function b64(string $value): string
    {
        return rtrim(strtr(base64_encode($value), '+/', '-_'), '=');
    }

    private static function unb64(string $value): ?string
    {
        if (!preg_match('/^[A-Za-z0-9_-]+$/', $value)) return null;
        $decoded = base64_decode(strtr($value, '-_', '+/'), true);
        return $decoded === false ? null : $decoded;
    }
}
