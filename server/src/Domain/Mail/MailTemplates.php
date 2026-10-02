<?php
declare(strict_types=1);

namespace Scutoons\Domain\Mail;

use InvalidArgumentException;

/**
 * The registry of every email the platform sends (agents/PLATFORM.md "Emails"). The API is the only sender; a template
 * is used through Mailer::send($id, $to, $locale, $vars).
 *
 * A template = an id (const below) + a definition:
 *   audience     member (to a reader/account) | admin (to MailRecipients::admins())
 *   description  one line for the admin's Emails page: when it is sent, by what
 *   vars         name → type: string | url | int | bool | ?string (optional, null) – `{name}` in the texts; bool and
 *                ?string may be left out, the rest is required (InvalidArgumentException otherwise)
 *   sample       values for the admin preview (every var)
 *   blocks       the body in order: ['p', key] a paragraph · ['button', key, 'url' => var] a link button (the text part
 *                prints the URL) · ['code', key, 'value' => var] a label + a big code; `'if' => var` = only when that
 *                var is true / not null / not ''
 *   fineprint    keys of the small print under the body (joined by a line break)
 * Texts (subject + every key, de and en): templates/mail/texts.php → [locale][id][key]. HTML frame: layout.html.php.
 *
 * Adding a template: a const + a definition here, its texts in both locales of texts.php, a call to
 * Mailer::send() (or sendToAdmins() for audience admin), a render test in tests/Unit/Domain/MailTest.php.
 * Planned (not registered until they are sent – same pattern):
 *   ownership_claim_received   to the claimant: we got your ownership claim
 *   ownership_claim_admin      to the admins (sendToAdmins): a new ownership claim to review
 *   ownership_transferred_from to the previous owner: the code moved to another account
 *   ownership_transferred_to   to the new owner: the code is now in your account
 *   ownership_transfer_offer   the owner sends a code to another person's email
 */
final class MailTemplates
{
    public const LOGIN_CODE = 'login_code';
    public const SIGNUP_WITHOUT_CODE = 'signup_without_code';

    public const LOCALES = ['de', 'en'];
    public const DEFAULT_LOCALE = 'en';

    /** @var array<string, array{audience: string, description: string, vars: array<string, string>, sample: array<string, mixed>, blocks: list<array<int|string, string>>, fineprint: list<string>}> */
    private const DEFINITIONS = [
        self::LOGIN_CODE => [
            'audience' => 'member',
            'description' => 'Sign-in: magic link + 6-digit one-time code (POST /auth/request/ for an account, or a new one with a signup code)',
            'vars' => ['link' => 'url', 'otp' => 'string', 'minutes' => 'int', 'code' => '?string', 'signup' => 'bool'],
            'sample' => ['link' => 'https://scutoons.com/?login=SAMPLE-TOKEN', 'otp' => '123456', 'minutes' => 30, 'code' => 'Ab3dE6gH', 'signup' => true],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'signup', 'if' => 'signup'],
                ['p', 'claim', 'if' => 'code'],
                ['button', 'button', 'url' => 'link'],
                ['code', 'otp', 'value' => 'otp'],
            ],
            'fineprint' => ['validity', 'ignore'],
        ],
        self::SIGNUP_WITHOUT_CODE => [
            'audience' => 'member',
            'description' => 'Sign-in request for an address without an account and without a usable signup code: how to get an account (no link, no code)',
            'vars' => ['url' => 'url', 'code' => '?string'],
            'sample' => ['url' => 'https://scutoons.com', 'code' => 'Ab3dE6gH'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'code', 'if' => 'code'],
                ['p', 'how'],
                ['p', 'read'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => ['ignore'],
        ],
    ];

    /** @return list<string> every template id */
    public static function ids(): array
    {
        return array_keys(self::DEFINITIONS);
    }

    public static function exists(string $id): bool
    {
        return isset(self::DEFINITIONS[$id]);
    }

    /** @return array{audience: string, description: string, vars: array<string, string>, sample: array<string, mixed>, blocks: list<array<int|string, string>>, fineprint: list<string>} */
    public static function get(string $id): array
    {
        return self::DEFINITIONS[$id] ?? throw new InvalidArgumentException("Unknown email template: $id");
    }

    /** de or en (default) from a locale like "de-AT"; anything else → en */
    public static function locale(mixed $value): string
    {
        if (!is_string($value)) return self::DEFAULT_LOCALE;
        $locale = strtolower(substr(trim($value), 0, 2));
        return in_array($locale, self::LOCALES, true) ? $locale : self::DEFAULT_LOCALE;
    }
}
