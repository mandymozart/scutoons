<?php
declare(strict_types=1);

namespace Scutoons\Domain\Newsletter;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;

/**
 * The personal button of a newsletter's `{signin}` paragraph (agents/NEWSLETTER.md "Writing a newsletter", #65; owner
 * 2026-10-20: one click = signed in, the text per recipient):
 *   - an account → "Anmelden" / "Sign in"
 *   - no account, but a QR code of the old form waits for this address (CodeService::legacySignupCode) → "Account
 *     anlegen – dein QR-Code wartet"; signing in creates the account and claims that code (LoginService::confirm)
 *   - no account and no such code → null (the paragraph is left out – an account needs a signup code)
 * The link is a login request (login_requests) whose token is mailed once: `APP_URL[/en]/?signin=<token>`, single use,
 * valid NEWSLETTER_SIGNIN_DAYS. The viewer asks once before it signs in (mail scanners open links). It has no 6-digit
 * code and doesn't count for the hourly login limits (its `ip_hash` is IP_HASH). Staff accounts (any role beyond
 * member) get the account page instead – a forwarded newsletter must never open an admin's account.
 */
final class NewsletterSignin
{
    /** marks the newsletter's login requests (a real ip_hash is an HMAC of an address – never this) */
    public const IP_HASH = 'newsletter-signin-0000000000000000000000000000000000000000000000';

    private const TEXTS = [
        'de' => ['signin' => 'Anmelden', 'signup' => 'Account anlegen – dein QR-Code wartet'],
        'en' => ['signin' => 'Sign in', 'signup' => 'Create your account – your QR code is waiting'],
    ];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly Users $users,
        private readonly Roles $roles,
        private readonly CodeService $codes,
    ) {
    }

    /**
     * The button for one recipient; makes its login request
     * @return array{text: string, url: string}|null
     */
    public function button(string $email, string $locale): ?array
    {
        $locale = MailTemplates::locale($locale);
        $email = strtolower(trim($email));
        $user = $this->users->byEmail($email);
        $texts = self::TEXTS[$locale];
        if ($user !== null) {
            if (array_diff($this->roles->of((string) $user['id']), [Roles::MEMBER]) !== []) {
                return ['text' => $texts['signin'], 'url' => $this->viewer($locale) . 'account'];
            }
            return ['text' => $texts['signin'], 'url' => $this->link($email, $locale, null)];
        }
        $code = $this->codes->legacySignupCode($email);
        if ($code === null) return null;
        return ['text' => $texts['signup'], 'url' => $this->link($email, $locale, $code)];
    }

    /**
     * The button as the preview shows it: a member's, pointing to the account page (no login request)
     * @return array{text: string, url: string}
     */
    public static function sample(string $locale, string $appUrl): array
    {
        $locale = MailTemplates::locale($locale);
        return ['text' => self::TEXTS[$locale]['signin'], 'url' => rtrim($appUrl, '/') . ($locale === 'en' ? '/en' : '') . '/account'];
    }

    private function link(string $email, string $locale, ?string $code): string
    {
        $id = Ids::uuid7($this->clock->ms());
        $token = Ids::token();
        $days = max(1, $this->settings->int('NEWSLETTER_SIGNIN_DAYS'));
        $this->db->run(
            'INSERT INTO login_requests (id, email, token_hash, code_hash, locale, claim_code, claim_item_id, claim_variant, marketing_opt_in, ip_hash, attempts, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, 0, ?, 0, ?, ?)',
            // no 6-digit code: the code hash belongs to a value nobody knows
            [$id, $email, hash('sha256', $token), hash('sha256', Ids::token()), $locale, $code, self::IP_HASH,
                $this->clock->utc(), $this->clock->utc($days * 86400)]);
        return $this->viewer($locale) . '?signin=' . rawurlencode($token);
    }

    /** The viewer's root in the recipient's language (de = the base locale, no prefix) */
    private function viewer(string $locale): string
    {
        return rtrim($this->settings->get('APP_URL'), '/') . ($locale === 'en' ? '/en/' : '/');
    }
}
