<?php
declare(strict_types=1);

namespace Scutoons\Domain\Mail;

use InvalidArgumentException;

/**
 * The registry of every email the platform sends (agents/PLATFORM.md "Emails"). The API is the only sender; a template
 * is used through Mailer::send($id, $to, $locale, $vars).
 *
 * A template = an id (const below) + a definition:
 *   audience     member (to a reader/account) | admin (to MailRecipients::admins()) | creator (creator app accounts)
 *   description  one line for the admin's Emails page: when it is sent, by what
 *   vars         name → type: string | url | int | bool | ?string (optional, null) – `{name}` in the texts; bool and
 *                ?string may be left out, the rest is required (InvalidArgumentException otherwise)
 *   sample       values for the admin preview (every var)
 *   blocks       the body in order: ['p', key] a paragraph · ['lines', key] a paragraph whose line breaks stay (lists)
 *                · ['button', key, 'url' => var] a link button (the text part prints the URL) · ['code', key, 'value' =>
 *                var] a label + a big code; `'if' => var` = only when that var is true / not null / not ''
 *   fineprint    keys of the small print under the body (joined by a line break)
 * Texts (subject + every key, de and en): templates/mail/texts.php → [locale][id][key]. HTML frame: layout.html.php.
 *
 * Adding a template: a const + a definition here, its texts in both locales of texts.php, a call to
 * Mailer::send() (or sendToAdmins() for audience admin), a render test in tests/Unit/Domain/MailTest.php.
 * Ownership + gifting (agents/ITEMS.md "Ownership, claims, gifting"; Domain\Ownership): ownership_claim_received,
 * ownership_claim_admin, ownership_claim_rejected, ownership_transferred_from, ownership_transferred_to, gift_received.
 * Review before publish (Domain\Content\Reviews, audience creator): review_requested (reviewers), review_published,
 * review_returned (the author) – one mail per person per submit / decision, listing every file.
 * Embassy (Domain\Embassy\EmbassyReminders, audience embassy): embassy_portion – a team member's own reminder (weekly
 * or daily) of their portion; never to a contact.
 * Planned (not registered until it is sent – same pattern):
 *   ownership_transfer_offer   the owner sends a code to another person's email
 */
final class MailTemplates
{
    public const LOGIN_CODE = 'login_code';
    public const SIGNUP_WITHOUT_CODE = 'signup_without_code';
    public const OWNERSHIP_CLAIM_RECEIVED = 'ownership_claim_received';
    public const OWNERSHIP_CLAIM_ADMIN = 'ownership_claim_admin';
    public const OWNERSHIP_CLAIM_REJECTED = 'ownership_claim_rejected';
    public const OWNERSHIP_TRANSFERRED_FROM = 'ownership_transferred_from';
    public const OWNERSHIP_TRANSFERRED_TO = 'ownership_transferred_to';
    public const GIFT_RECEIVED = 'gift_received';
    public const REVIEW_REQUESTED = 'review_requested';
    public const REVIEW_PUBLISHED = 'review_published';
    public const REVIEW_RETURNED = 'review_returned';
    public const EMBASSY_PORTION = 'embassy_portion';
    public const AI_TOKEN_EXPIRING = 'ai_token_expiring';

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
        self::OWNERSHIP_CLAIM_RECEIVED => [
            'audience' => 'member',
            'description' => 'Ownership claim filed ("This is mine", POST /claims/): to the claimant – we got it, an admin looks at it',
            'vars' => ['code' => 'string', 'url' => 'url'],
            'sample' => ['code' => 'Ab3dE6gH', 'url' => 'https://scutoons.com/account'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'next'],
                ['p', 'proof'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => ['ignore'],
        ],
        self::OWNERSHIP_CLAIM_ADMIN => [
            'audience' => 'admin',
            'description' => 'Ownership claim filed: to the admin recipients – a new claim to review (admin app → Claims)',
            'vars' => ['code' => 'string', 'claimant' => 'string', 'statement' => 'string', 'url' => 'url'],
            'sample' => ['code' => 'Ab3dE6gH', 'claimant' => 'reader@example.com', 'statement' => 'I bought the hoodie at the Vienna show, the code was registered before I got home.', 'url' => 'https://admin.scutoons.com/claims/00000000-0000-0000-0000-000000000000'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'statement'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => ['files'],
        ],
        self::OWNERSHIP_CLAIM_REJECTED => [
            'audience' => 'member',
            'description' => 'Ownership claim rejected by an admin: to the claimant (the admin note stays internal)',
            'vars' => ['code' => 'string', 'url' => 'url'],
            'sample' => ['code' => 'Ab3dE6gH', 'url' => 'https://scutoons.com/account'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'more'],
                ['p', 'read'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => [],
        ],
        self::OWNERSHIP_TRANSFERRED_FROM => [
            'audience' => 'member',
            'description' => 'A code moved to another account (an admin decided a claim, or an admin transfer): to the previous owner',
            'vars' => ['code' => 'string', 'item' => 'string', 'url' => 'url'],
            'sample' => ['code' => 'Ab3dE6gH', 'item' => 'Dago Hoodie #12', 'url' => 'https://scutoons.com/account'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'read'],
                ['p', 'wrong'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => [],
        ],
        self::OWNERSHIP_TRANSFERRED_TO => [
            'audience' => 'member',
            'description' => 'A code moved into this account (claim decided for the claimant, or an admin transfer): to the new owner',
            'vars' => ['code' => 'string', 'item' => 'string', 'url' => 'url'],
            'sample' => ['code' => 'Ab3dE6gH', 'item' => 'Dago Hoodie #12', 'url' => 'https://scutoons.com/account'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'certificate'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => [],
        ],
        self::GIFT_RECEIVED => [
            'audience' => 'member',
            'description' => 'A reader gifted an item (POST /inventory/gift/): to the recipient',
            'vars' => ['from' => 'string', 'item' => 'string', 'qty' => 'int', 'url' => 'url'],
            'sample' => ['from' => 'friend@example.com', 'item' => 'Gurke', 'qty' => 3, 'url' => 'https://scutoons.com/account'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'where'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => ['free'],
        ],
        self::REVIEW_REQUESTED => [
            'audience' => 'creator',
            'description' => 'A creator submitted drafts for review (POST /api/reviews): to every reviewer of the toon (admin → user → review rules)',
            'vars' => ['author' => 'string', 'toon' => 'string', 'files' => 'string', 'note' => '?string', 'url' => 'url'],
            'sample' => ['author' => 'writer@example.com', 'toon' => 'Cat Pope', 'files' => 'chapter 2 (de, en), scene 2/kiosk (de, en)', 'note' => 'The kiosk scene is new, the rest are typo fixes.', 'url' => 'https://creator.scutoons.com/'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'files'],
                ['p', 'note', 'if' => 'note'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => ['why'],
        ],
        self::REVIEW_PUBLISHED => [
            'audience' => 'creator',
            'description' => 'A reviewer published submitted drafts (POST /api/reviews/publish): to the author',
            'vars' => ['reviewer' => 'string', 'toon' => 'string', 'files' => 'string', 'note' => '?string', 'url' => 'url'],
            'sample' => ['reviewer' => 'editor@example.com', 'toon' => 'Cat Pope', 'files' => 'chapter 2 (de, en)', 'note' => 'Lovely – thanks!', 'url' => 'https://creator.scutoons.com/'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'files'],
                ['p', 'note', 'if' => 'note'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => [],
        ],
        self::REVIEW_RETURNED => [
            'audience' => 'creator',
            'description' => 'A reviewer sent submitted drafts back (POST /api/reviews/return): to the author',
            'vars' => ['reviewer' => 'string', 'toon' => 'string', 'files' => 'string', 'note' => '?string', 'url' => 'url'],
            'sample' => ['reviewer' => 'editor@example.com', 'toon' => 'Cat Pope', 'files' => 'chapter 2 (de, en)', 'note' => 'The cat would never say that – see slice 4.', 'url' => 'https://creator.scutoons.com/'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'files'],
                ['p', 'note', 'if' => 'note'],
                ['p', 'again'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => [],
        ],
        self::EMBASSY_PORTION => [
            'audience' => 'embassy',
            'description' => 'Embassy reminder (cron GET /api/embassy/cron, weekly on Monday or daily Mon–Fri as the person chose): to a team member – their portion of contacts, never to a contact',
            'vars' => ['done' => 'int', 'pace' => 'int', 'items' => '?string', 'more' => '?string', 'daily' => 'bool', 'weekly' => 'bool', 'checkin' => 'bool', 'url' => 'url'],
            'sample' => ['done' => 3, 'pace' => 10, 'items' => "• Rote Fabrik · Lea – mail 6 days ago, no answer yet (“Top Toy in Zürich”)
• Dachstock – voicemail 3 days ago – try again
• Openground – no contact yet – first mail",
                'more' => '4', 'daily' => false, 'weekly' => true, 'checkin' => true, 'url' => 'https://embassy.scutoons.com/'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'week', 'if' => 'weekly'],
                ['p', 'today', 'if' => 'daily'],
                ['lines', 'items', 'if' => 'items'],
                ['p', 'more', 'if' => 'more'],
                ['p', 'checkin', 'if' => 'checkin'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => ['change'],
        ],
        self::AI_TOKEN_EXPIRING => [
            'audience' => 'member',
            'description' => 'An AI connection (personal token, Domain\\Auth\\ApiTokens) is used within its last 14 days: to its member, once per period – renew it on the account page',
            'vars' => ['name' => 'string', 'date' => 'string', 'url' => 'url'],
            'sample' => ['name' => 'Claude on my laptop', 'date' => '2027-04-09', 'url' => 'https://scutoons.com/account'],
            'blocks' => [
                ['p', 'intro'],
                ['p', 'how'],
                ['button', 'button', 'url' => 'url'],
            ],
            'fineprint' => ['why'],
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
