<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Infrastructure\Database\Database;

/**
 * The account filters of the admin (agents/research/GIFT-INVITES.md 1b; owner 2026-10-14: "we need filters for
 * these"): one WHERE for the Users list, a campaign's account list, a segment's "add all matching" – over `users u`.
 *   q              part of the address
 *   role           a role name · opt_in 1|0
 *   signup         membership | promo | invite (a gift invite's promo code) | none | another code type – the code the
 *                  account was made with
 *   edition        an edition id – owns a code of it or signed up with one
 *   batch          a batch (manifest file) – owns a code of it or signed up with one (promo batches too)
 *   campaign       a campaign id, with campaign_role invited (invited someone) | joined (joined by an invite) | both
 *   segment        a segment id (none = in no segment)
 */
final class AccountFilters
{
    public const NAMES = ['role', 'opt_in', 'signup', 'edition', 'batch', 'campaign', 'campaign_role', 'segment'];

    /**
     * @param array<string, string|null> $filters
     * @return array{0: string, 1: list<mixed>} the WHERE clause ('' = none) and its parameters
     */
    public static function where(string $q, array $filters): array
    {
        $conditions = [];
        $params = [];
        $f = static fn(string $name): ?string => isset($filters[$name]) && $filters[$name] !== '' ? (string) $filters[$name] : null;
        if ($q !== '') {
            $conditions[] = "u.email LIKE ? ESCAPE '!'";
            $params[] = Database::likeContains(strtolower($q));
        }
        if (($role = $f('role')) !== null) {
            $conditions[] = 'EXISTS (SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = u.id AND r.name = ?)';
            $params[] = $role;
        }
        if (($optIn = $f('opt_in')) !== null) {
            $conditions[] = 'u.marketing_opt_in = ?';
            $params[] = Users::flag($optIn) ? 1 : 0;
        }
        if (($signup = $f('signup')) !== null) {
            $invite = 'EXISTS (SELECT 1 FROM campaign_invites ci WHERE ci.invite_code = u.signup_code)';
            if ($signup === 'none') $conditions[] = "(u.signup_code IS NULL OR u.signup_code = '')";
            elseif ($signup === 'invite') $conditions[] = $invite;
            else {
                $conditions[] = "EXISTS (SELECT 1 FROM qr sq WHERE sq.code = u.signup_code AND sq.type = ?)" . ($signup === 'promo' ? " AND NOT $invite" : '');
                $params[] = $signup;
            }
        }
        foreach (['edition' => 'edition_id', 'batch' => 'batch'] as $name => $column) {
            if (($value = $f($name)) === null) continue;
            $conditions[] = "EXISTS (SELECT 1 FROM qr bq WHERE bq.$column = ? AND (bq.claimed_by = u.id OR bq.code = u.signup_code))";
            $params[] = $value;
        }
        if (($campaign = $f('campaign')) !== null) {
            $role = $f('campaign_role');
            $who = $role === 'invited' ? 'ci2.inviter_id = u.id' : ($role === 'joined' ? 'ci2.joined_user_id = u.id' : '(ci2.inviter_id = u.id OR ci2.joined_user_id = u.id)');
            $conditions[] = "EXISTS (SELECT 1 FROM campaign_invites ci2 WHERE ci2.campaign_id = ? AND $who)";
            $params[] = $campaign;
        }
        if (($segment = $f('segment')) !== null) {
            if ($segment === 'none') $conditions[] = 'NOT EXISTS (SELECT 1 FROM segment_members sm WHERE sm.user_id = u.id)';
            else {
                $conditions[] = 'EXISTS (SELECT 1 FROM segment_members sm WHERE sm.segment_id = ? AND sm.user_id = u.id)';
                $params[] = $segment;
            }
        }
        return [$conditions ? 'WHERE ' . implode(' AND ', $conditions) : '', $params];
    }

    /**
     * The filters of a request body ({ q?, filters?: { … } }) – for "add all matching"
     * @param array<string, mixed> $body
     * @return array{0: string, 1: array<string, string|null>}
     */
    public static function fromBody(array $body): array
    {
        $q = is_string($body['q'] ?? null) ? trim($body['q']) : '';
        $filters = [];
        foreach (is_array($body['filters'] ?? null) ? $body['filters'] : [] as $name => $value) {
            if (in_array($name, self::NAMES, true) && (is_string($value) || is_int($value))) $filters[$name] = (string) $value;
        }
        return [$q, $filters];
    }
}
