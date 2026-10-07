<?php
declare(strict_types=1);

namespace Scutoons\Domain\Newsletter;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Infrastructure\Database\Database;

/**
 * Who a newsletter goes to (agents/NEWSLETTER.md "Audiences"). A newsletter picks one or more:
 *   members_opted_in   accounts with "Send me news" (users.marketing_opt_in)
 *   members_all        every account
 *   contacts           every contact (mail_contacts: the old QR form + the event lists)
 *   contacts:<source>  the contacts of one source, e.g. contacts:qr-claim
 * Each address once (members first: an account's language and name win over its contact row), never a suppressed one
 * (mail_suppressions), never an invalid one. Contacts without a language are listed with locale null – a newsletter
 * can't start while it has one.
 */
final class Audiences
{
    public const MEMBERS_OPTED_IN = 'members_opted_in';
    public const MEMBERS_ALL = 'members_all';
    public const CONTACTS = 'contacts';

    public function __construct(private readonly Database $db, private readonly Contacts $contacts)
    {
    }

    /**
     * Every audience with how many it reaches now (suppressed and invalid addresses left out)
     * @return list<array{id: string, kind: string, source: ?string, count: int, missing_locale: int}>
     */
    public function all(): array
    {
        $items = [];
        $ids = [self::MEMBERS_OPTED_IN, self::MEMBERS_ALL, self::CONTACTS];
        foreach ($this->contacts->sources() as $source) $ids[] = self::CONTACTS . ':' . $source['source'];
        foreach ($ids as $id) {
            $recipients = $this->recipients([$id]);
            $items[] = [
                'id' => $id,
                'kind' => str_starts_with($id, 'members') ? 'members' : 'contacts',
                'source' => str_starts_with($id, self::CONTACTS . ':') ? substr($id, strlen(self::CONTACTS) + 1) : null,
                'count' => count($recipients),
                'missing_locale' => count(array_filter($recipients, fn(array $r) => $r['locale'] === null)),
            ];
        }
        return $items;
    }

    /**
     * Checks a newsletter's audiences: a non-empty list of known ids (a contacts:<source> may name a source without
     * contacts – it is simply empty)
     * @return list<string>
     */
    public static function check(mixed $audiences): array
    {
        if (!is_array($audiences) || !array_is_list($audiences)) throw new ApiError(400, 'invalid_audiences', 'audiences: a list of audience ids.');
        $clean = [];
        foreach ($audiences as $id) {
            $ok = is_string($id) && (in_array($id, [self::MEMBERS_OPTED_IN, self::MEMBERS_ALL, self::CONTACTS], true)
                || (str_starts_with($id, self::CONTACTS . ':') && preg_match(Contacts::SOURCE_PATTERN, substr($id, strlen(self::CONTACTS) + 1))));
            if (!$ok) throw new ApiError(400, 'invalid_audiences', 'Unknown audience: ' . (is_scalar($id) ? (string) $id : '?'));
            if (!in_array($id, $clean, true)) $clean[] = $id;
        }
        return $clean;
    }

    /**
     * The recipients of these audiences, each address once, members first
     * @param list<string> $audiences
     * @return list<array{email: string, locale: ?string, name: ?string, source: string}>
     */
    public function recipients(array $audiences): array
    {
        $byEmail = [];
        $add = static function (array $rows) use (&$byEmail): void {
            foreach ($rows as $row) {
                $email = Users::normalizeEmail($row['email']);
                if ($email === null || isset($byEmail[$email])) continue;
                $member = $row['user_id'] !== null;
                $byEmail[$email] = [
                    'email' => $email,
                    'locale' => $member ? MailTemplates::locale((string) $row['user_locale']) : ($row['locale'] !== null ? (string) $row['locale'] : null),
                    'name' => self::name($member ? $row['user_name'] : $row['name']),
                    'source' => $member ? 'member' : 'contact',
                ];
            }
        };
        $members = 'SELECT u.email, u.id AS user_id, u.locale AS user_locale, u.display_name AS user_name, NULL AS locale, NULL AS name FROM users u
            LEFT JOIN mail_suppressions s ON s.email = u.email WHERE s.email IS NULL';
        $contacts = 'SELECT c.email, u.id AS user_id, u.locale AS user_locale, u.display_name AS user_name, c.locale, c.name FROM mail_contacts c
            LEFT JOIN users u ON u.email = c.email LEFT JOIN mail_suppressions s ON s.email = c.email WHERE s.email IS NULL';
        // members before contacts, whatever order the newsletter lists them in
        if (in_array(self::MEMBERS_ALL, $audiences, true)) $add($this->db->all("$members ORDER BY u.created_at, u.email"));
        elseif (in_array(self::MEMBERS_OPTED_IN, $audiences, true)) $add($this->db->all("$members AND u.marketing_opt_in = 1 ORDER BY u.created_at, u.email"));
        if (in_array(self::CONTACTS, $audiences, true)) {
            $add($this->db->all("$contacts ORDER BY c.created_at, c.id"));
        } else {
            $sources = [];
            foreach ($audiences as $id) {
                if (str_starts_with($id, self::CONTACTS . ':')) $sources[] = substr($id, strlen(self::CONTACTS) + 1);
            }
            if ($sources !== []) {
                $in = implode(', ', array_fill(0, count($sources), '?'));
                $add($this->db->all("$contacts AND c.source IN ($in) ORDER BY c.created_at, c.id", $sources));
            }
        }
        return array_values($byEmail);
    }

    private static function name(mixed $value): ?string
    {
        return is_string($value) && trim($value) !== '' ? trim($value) : null;
    }
}
