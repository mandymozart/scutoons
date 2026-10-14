<?php
declare(strict_types=1);

namespace Scutoons\Domain\Campaigns;

use PDOException;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Mail\MailFailed;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\Ownership\Gifts;
use Scutoons\Domain\Ownership\OwnershipTransfers;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;

/**
 * Gift invites (agents/research/GIFT-INVITES.md, FEATURES 10.96; owner 2026-10-13/14): while a campaign runs for a
 * member (CampaignStore::runningFor), a gift to an address without an account becomes an invite.
 *
 * - The gift: the member's own virtual item (taken from their inventory at once – Gifts::takeable / takeOut, the same
 *   rules as a gift; a piece with a code goes only to an existing account) or the campaign's item (given by SCUTOONS,
 *   counted against the campaign's stock). Limits: the campaign's per member / per day, GIFT_MAX_PER_DAY together with
 *   gifts, one open invite per address, the recipient lookups of Gifts.
 * - A physical campaign item the member pays: status `payment` – the admins get a mail, an admin sends the member a
 *   payment link or the bank details by hand and marks it paid (markPaid); not paid in wait_days → `unpaid`.
 * - Sent (`waiting`): a personal promo code (one per invite; "is it really different to promo?" – no) is mailed to the
 *   friend with the inviter's address; the invite waits until expires_at (wait_days, never past the campaign's end),
 *   the code accepts a signup only while its invite waits (CodeService::signupRefusal → `used`).
 * - Joining (deliverTo, LoginService::confirm – also with another code, by email): every waiting invite of the address
 *   is delivered as a gift from the inviter (transfer log kind gift → the "X schenkt dir Y" notice); a physical one asks
 *   the friend for a shipping address (setAddress) and an admin ships it (markShipped). The invite code's end becomes
 *   the campaign's reading rule: keep → none (the twist: invitees keep reading after the campaign), until → that date.
 * - Recall (member, only while not accepted: payment | waiting), return (sweep: not joined in time → the item back to
 *   the member, a mail) – run by GET /api/invites, the admin lists and the cron URL.
 */
final class Invites
{
    public const OPEN = ['payment', 'waiting'];
    public const STATUSES = ['payment', 'waiting', 'joined', 'returned', 'recalled', 'unpaid'];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Users $users,
        private readonly Catalog $catalog,
        private readonly InventoryService $inventory,
        private readonly OwnershipTransfers $transfers,
        private readonly Gifts $gifts,
        private readonly CampaignStore $campaigns,
        private readonly Mailer $mailer,
        private readonly Settings $settings,
    ) {
    }

    // ── The member ───────────────────────────────────────────────────────────────────────────────────────────

    /**
     * GET /api/invites: the campaign the member may invite with (null = none), their invites (open first, then the last
     * 20 others) and the gifts waiting for their shipping address (invites they joined by).
     * @param array<string, mixed> $user
     * @return array{campaign: ?array<string, mixed>, invites: list<array<string, mixed>>, address_needed: list<array<string, mixed>>}
     */
    public function mine(array $user): array
    {
        $offer = $this->campaigns->offerFor($user);
        try {
            $this->sweep();
            $userId = (string) $user['id'];
            $rows = $this->db->all("SELECT * FROM campaign_invites WHERE inviter_id = ? ORDER BY (status IN ('payment', 'waiting')) DESC, created_at DESC LIMIT 40", [$userId]);
            $address = $this->db->all('SELECT i.*, c.countries FROM campaign_invites i JOIN campaigns c ON c.id = i.campaign_id
                WHERE i.joined_user_id = ? AND i.physical = 1 AND i.ship_given_at IS NULL ORDER BY i.joined_at', [$userId]);
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            return ['campaign' => null, 'invites' => [], 'address_needed' => []];
        }
        $locale = MailTemplates::locale($user['locale'] ?? null);
        return [
            'campaign' => $offer,
            'invites' => array_map(fn(array $r) => $this->memberJson($r, $locale), $rows),
            'address_needed' => array_map(fn(array $r) => [
                'id' => (string) $r['id'],
                'item_id' => (string) $r['item_id'],
                'variant' => $r['variant'],
                'item' => $this->transfers->itemName((string) $r['item_id'], $r['variant'] !== null ? (string) $r['variant'] : null, $locale),
                'countries' => $r['countries'] !== null && $r['countries'] !== '' ? explode(',', (string) $r['countries']) : [],
            ], $address),
        ];
    }

    /**
     * POST /api/invites { to, entry_id | item_id (+ variant), qty? } – the gift is the campaign's: no entry for a
     * campaign item. 403 no_campaign · 400 invalid_email / self_gift / code_needs_account (+ Gifts::prepare) ·
     * 409 has_account / already_invited · 429 daily_limit / campaign_limit / rate_limited · 410 sold_out.
     * @param array<string, mixed> $user
     * @param array<string, mixed> $body
     * @return array<string, mixed> the invite (memberJson)
     */
    public function create(array $user, array $body): array
    {
        $this->campaigns->requireTables();
        $userId = (string) $user['id'];
        $email = Users::normalizeEmail($body['to'] ?? null) ?? throw new ApiError(400, 'invalid_email', 'to: the email address of the person you want to invite.');
        if ($email === strtolower((string) $user['email'])) throw new ApiError(400, 'self_gift', 'You cannot invite yourself.');
        $c = $this->campaigns->runningFor($user) ?? throw new ApiError(403, 'no_campaign', 'Invites are not open right now.');
        $offer = $this->campaigns->offerFor($user) ?? throw new ApiError(403, 'no_campaign', 'Invites are not open right now.');

        $own = null;
        if ($c['gift'] === 'own') {
            $own = $this->gifts->prepare($userId, $body);
            if ($own['code'] !== null) throw new ApiError(400, 'code_needs_account', 'A piece with a code can only go to someone who has an account.');
        }
        if ($offer['left'] < 1) {
            if ($this->campaigns->stockLeft($c) === 0) throw new ApiError(410, 'sold_out', 'The campaign has given away everything.');
            throw new ApiError(429, 'campaign_limit', 'You have invited as many people as this campaign allows.', ['per_member' => (int) $c['per_member'], 'per_day' => (int) $c['per_day']]);
        }
        $this->gifts->checkDailyLimit($userId);
        $this->gifts->lookup($userId);
        if ($this->users->byEmail($email) !== null) throw new ApiError(409, 'has_account', 'This person has an account already – give it to them directly.');

        $item = $own !== null ? $own['item'] : ($this->catalog->item((string) $c['item_id']) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.'));
        $physical = $own === null && CampaignStore::isPhysical($item);
        $payment = $physical && $c['payer'] === 'member';
        $now = $this->clock->utc();
        $id = Ids::uuid();
        $locale = MailTemplates::locale($user['locale'] ?? null);

        $this->db->transaction(function () use ($id, $c, $userId, $email, $locale, $own, $physical, $payment, $now) {
            // one open invite per address (locked read: two members inviting the same friend at once)
            $open = $this->db->value("SELECT 1 FROM campaign_invites WHERE email = ? AND status IN ('payment', 'waiting') LIMIT 1 FOR UPDATE", [$email]);
            if ($open !== null) throw new ApiError(409, 'already_invited', 'Someone has invited this person already – they have an invite waiting.');
            if ($own !== null) {
                $entry = $this->gifts->takeable($userId, $own['entry'], $own['qty']);
                $this->gifts->takeOut($userId, $entry, $own['qty']);
                $itemId = (string) $entry['item_id'];
                $variant = $entry['variant'] !== null ? (string) $entry['variant'] : null;
                $qty = $own['qty'];
            } else {
                $stock = $this->campaigns->stockLeft($this->db->one('SELECT * FROM campaigns WHERE id = ? FOR UPDATE', [$c['id']]) ?? $c);
                if ($stock === 0) throw new ApiError(410, 'sold_out', 'The campaign has given away everything.');
                $itemId = (string) $c['item_id'];
                $variant = $c['variant'] !== null ? (string) $c['variant'] : null;
                $qty = 1;
            }
            $this->db->run('INSERT INTO campaign_invites (id, campaign_id, inviter_id, email, locale, gift, item_id, variant, qty, physical, status, price_cents, currency, created_at, payment_due_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', [
                $id, $c['id'], $userId, $email, $locale, $own !== null ? 'own' : 'campaign', $itemId, $variant, $qty, $physical ? 1 : 0,
                $payment ? 'payment' : 'waiting', $payment ? (int) $c['price_cents'] : null, $payment ? (string) $c['currency'] : null,
                $now, $payment ? CampaignStore::waitUntil($c, $this->clock->time()) : null,
            ]);
        });

        if ($payment) {
            $row = $this->row($id);
            $this->mailer->sendToAdmins(MailTemplates::INVITE_PAYMENT_REQUESTED, [
                'member' => (string) $user['email'],
                'friend' => $email,
                'item' => $this->itemName($row, 'de'),
                'price' => self::price((int) $row['price_cents'], (string) $row['currency']),
                'until' => self::date((string) $row['payment_due_at'], 'de'),
                'url' => rtrim($this->settings->get('ADMIN_URL'), '/') . '/campaigns/' . $c['id'] . '?tab=invites',
            ]);
        } else {
            $this->send($id);
        }
        return $this->memberJson($this->row($id), $locale);
    }

    /**
     * POST /api/invites/{id}/recall – only while not accepted (payment | waiting): the member's own item comes back,
     * the invite code stops working. 404 not_found · 409 not_open.
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function recall(array $user, string $id): array
    {
        $this->campaigns->requireTables();
        $locale = MailTemplates::locale($user['locale'] ?? null);
        $this->db->transaction(function () use ($user, $id) {
            $row = $this->db->one('SELECT * FROM campaign_invites WHERE id = ? AND inviter_id = ? FOR UPDATE', [$id, (string) $user['id']])
                ?? throw new ApiError(404, 'not_found', 'This invite does not exist.');
            if (!in_array($row['status'], self::OPEN, true)) throw new ApiError(409, 'not_open', 'This invite was accepted or has ended – it cannot be called back.', ['status' => $row['status']]);
            $this->end($row, 'recalled');
        });
        return $this->memberJson($this->row($id), $locale);
    }

    /**
     * PUT /api/invites/{id}/address { name, address, country } – the friend's shipping address for a physical gift
     * (they joined by it), until it is shipped. 404 not_found · 409 shipped · 400 invalid_address / invalid_country.
     * @param array<string, mixed> $user
     * @param array<string, mixed> $body
     * @return array<string, mixed> mine()
     */
    public function setAddress(array $user, string $id, array $body): array
    {
        $this->campaigns->requireTables();
        $row = $this->db->one('SELECT i.*, c.countries FROM campaign_invites i JOIN campaigns c ON c.id = i.campaign_id WHERE i.id = ? AND i.joined_user_id = ? AND i.physical = 1', [$id, (string) $user['id']])
            ?? throw new ApiError(404, 'not_found', 'There is no gift to send to you here.');
        if ($row['shipped_at'] !== null) throw new ApiError(409, 'shipped', 'It is on its way already.');
        $name = is_string($body['name'] ?? null) ? trim($body['name']) : '';
        $address = is_string($body['address'] ?? null) ? trim(str_replace("\r", '', $body['address'])) : '';
        if ($name === '' || mb_strlen($name) > 200 || $address === '' || mb_strlen($address) > 1000) throw new ApiError(400, 'invalid_address', 'Name and address, please.');
        $country = strtoupper(is_string($body['country'] ?? null) ? trim($body['country']) : '');
        $countries = $row['countries'] !== null && $row['countries'] !== '' ? explode(',', (string) $row['countries']) : [];
        if (!in_array($country, $countries, true)) throw new ApiError(400, 'invalid_country', 'We can only send it to these countries.', ['countries' => $countries]);
        $this->db->run('UPDATE campaign_invites SET ship_name = ?, ship_address = ?, ship_country = ?, ship_given_at = ? WHERE id = ?', [$name, $address, $country, $this->clock->utc(), $id]);
        return $this->mine($user);
    }

    // ── Joining (LoginService::confirm) ──────────────────────────────────────────────────────────────────────

    /**
     * Inside the signup transaction, right after the account was made: every waiting invite of the address is
     * delivered; $signupCode = the code the account was made with – an invite's code gets the campaign's reading rule.
     * Nothing before the migration.
     * @param array<string, mixed> $user the new users row
     * @return list<string> the invites delivered
     */
    public function deliverTo(array $user, ?string $signupCode): array
    {
        try {
            $rows = $this->db->all("SELECT i.*, c.access, c.access_until FROM campaign_invites i JOIN campaigns c ON c.id = i.campaign_id
                WHERE i.email = ? AND i.status = 'waiting' AND i.expires_at > ? ORDER BY i.created_at FOR UPDATE", [strtolower((string) $user['email']), $this->clock->utc()]);
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            return [];
        }
        $userId = (string) $user['id'];
        $now = $this->clock->utc();
        $done = [];
        foreach ($rows as $r) {
            $variant = $r['variant'] !== null ? (string) $r['variant'] : null;
            if ((int) $r['physical'] !== 1) {
                $item = $this->catalog->item((string) $r['item_id']);
                $qty = $item !== null ? min((int) $r['qty'], $this->inventory->room($userId, $item)) : 0;
                if ($qty > 0) $this->inventory->add($userId, (string) $r['item_id'], $variant, $qty, 'gift');
            }
            // the transfer log: the "X schenkt dir Y" notice (a physical one too – it is on its way)
            $this->transfers->record('gift', null, (string) $r['item_id'], $variant, (int) $r['qty'], (string) $r['inviter_id'], $userId, (string) $r['inviter_id'], null, 'invite ' . $r['id']);
            $this->db->run("UPDATE campaign_invites SET status = 'joined', joined_at = ?, joined_user_id = ? WHERE id = ?", [$now, $userId, $r['id']]);
            if ($signupCode !== null && $r['invite_code'] === $signupCode) {
                // the invitee's reading: keep → the code never ends; until → the campaign's date (qr times are local)
                $until = $r['access'] === 'until' && $r['access_until'] !== null ? date('Y-m-d H:i:s', (int) strtotime($r['access_until'] . ' UTC')) : null;
                $this->db->run('UPDATE qr SET expires_at = ? WHERE code = ?', [$until, $signupCode]);
            }
            $done[] = (string) $r['id'];
        }
        return $done;
    }

    // ── Time ─────────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * Ends what ran out: waiting past expires_at → returned (the member's item back, a mail to the member), payment past
     * payment_due_at → unpaid. GET /api/invites, the admin lists and the cron URL call it.
     * @return array{returned: int, unpaid: int}
     */
    public function sweep(): array
    {
        $now = $this->clock->utc();
        $due = $this->db->all("SELECT id FROM campaign_invites WHERE (status = 'waiting' AND expires_at <= ?) OR (status = 'payment' AND payment_due_at <= ?) LIMIT 200", [$now, $now]);
        $returned = 0;
        $unpaid = 0;
        foreach ($due as $d) {
            $ended = $this->db->transaction(function () use ($d, $now) {
                $row = $this->db->one('SELECT * FROM campaign_invites WHERE id = ? FOR UPDATE', [$d['id']]);
                if ($row === null) return null;
                if ($row['status'] === 'waiting' && $row['expires_at'] <= $now) return $this->end($row, 'returned');
                if ($row['status'] === 'payment' && $row['payment_due_at'] <= $now) return $this->end($row, 'unpaid');
                return null;
            });
            if ($ended === null) continue;
            if ($ended['status'] === 'unpaid') $unpaid++;
            else $returned++;
            $this->mailReturned($ended);
        }
        return ['returned' => $returned, 'unpaid' => $unpaid];
    }

    // ── Admin ────────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * The invites of a campaign (status filter; `to_ship` = joined physical with an address, not shipped).
     * @return list<array<string, mixed>>
     */
    public function forCampaign(string $campaignId, ?string $status = null): array
    {
        $this->campaigns->requireTables();
        $this->campaigns->row($campaignId);
        $this->sweep();
        $where = 'i.campaign_id = ?';
        $params = [$campaignId];
        if ($status === 'to_ship') $where .= " AND i.status = 'joined' AND i.physical = 1 AND i.shipped_at IS NULL";
        elseif ($status !== null && in_array($status, self::STATUSES, true)) {
            $where .= ' AND i.status = ?';
            $params[] = $status;
        }
        $rows = $this->db->all("SELECT i.*, u.email AS inviter_email, j.email AS joined_email FROM campaign_invites i
            LEFT JOIN users u ON u.id = i.inviter_id LEFT JOIN users j ON j.id = i.joined_user_id
            WHERE $where ORDER BY i.created_at DESC LIMIT 500", $params);
        return array_map(fn(array $r) => $this->adminJson($r), $rows);
    }

    /**
     * POST /admin/campaigns/invites/{id}/paid – the member paid: the invite goes out. 404 · 409 not_payment.
     * @param array<string, mixed> $admin
     * @return array<string, mixed>
     */
    public function markPaid(array $admin, string $id): array
    {
        $this->campaigns->requireTables();
        $this->db->transaction(function () use ($admin, $id) {
            $row = $this->db->one('SELECT * FROM campaign_invites WHERE id = ? FOR UPDATE', [$id]) ?? throw new ApiError(404, 'not_found', 'This invite does not exist.');
            if ($row['status'] !== 'payment') throw new ApiError(409, 'not_payment', 'This invite is not waiting for a payment.', ['status' => $row['status']]);
            $this->db->run("UPDATE campaign_invites SET status = 'waiting', paid_at = ?, paid_by = ? WHERE id = ?", [$this->clock->utc(), (string) $admin['id'], $id]);
        });
        $this->send($id);
        return $this->adminJson($this->adminRow($id));
    }

    /**
     * POST /admin/campaigns/invites/{id}/shipped { note? } – 404 · 409 not_shippable (not joined / no address).
     * @param array<string, mixed> $admin
     * @return array<string, mixed>
     */
    public function markShipped(array $admin, string $id, ?string $note): array
    {
        $this->campaigns->requireTables();
        $row = $this->row($id);
        if ($row['status'] !== 'joined' || (int) $row['physical'] !== 1 || $row['ship_given_at'] === null) throw new ApiError(409, 'not_shippable', 'No shipping address yet.');
        $note = $note !== null ? mb_substr(trim($note), 0, 500) : null;
        $this->db->run('UPDATE campaign_invites SET shipped_at = ?, shipped_by = ?, ship_note = ? WHERE id = ?', [$this->clock->utc(), (string) $admin['id'], $note !== '' ? $note : null, $id]);
        return $this->adminJson($this->adminRow($id));
    }

    /**
     * The campaign's numbers (owner: "who is using the campaign and who actually signed up eventually through their
     * invites"): totals and per member; joined_later = an address that made an account afterwards without its invite.
     * @return array<string, mixed>
     */
    public function stats(string $campaignId): array
    {
        $this->campaigns->requireTables();
        $this->campaigns->row($campaignId);
        $this->sweep();
        $rows = $this->db->all("SELECT i.inviter_id, u.email, COUNT(*) AS invites, SUM(i.status = 'joined') AS joined, SUM(i.status IN ('payment', 'waiting')) AS open,
                SUM(i.status IN ('returned', 'recalled', 'unpaid')) AS ended,
                SUM(i.status IN ('returned', 'recalled', 'unpaid') AND EXISTS (SELECT 1 FROM users x WHERE x.email = i.email AND x.created_at > i.created_at)) AS joined_later,
                MAX(i.created_at) AS last_invite
            FROM campaign_invites i LEFT JOIN users u ON u.id = i.inviter_id WHERE i.campaign_id = ? GROUP BY i.inviter_id, u.email ORDER BY invites DESC, last_invite DESC", [$campaignId]);
        $members = array_map(fn(array $r) => [
            'id' => (string) $r['inviter_id'], 'email' => $r['email'] !== null ? (string) $r['email'] : null, 'invites' => (int) $r['invites'], 'joined' => (int) $r['joined'],
            'open' => (int) $r['open'], 'ended' => (int) $r['ended'], 'joined_later' => (int) $r['joined_later'], 'last_invite' => Time::iso($r['last_invite']),
        ], $rows);
        $by = [];
        foreach ($this->db->all('SELECT status, COUNT(*) AS n FROM campaign_invites WHERE campaign_id = ? GROUP BY status', [$campaignId]) as $r) $by[(string) $r['status']] = (int) $r['n'];
        return [
            'inviters' => count($members),
            'sent' => array_sum($by),
            'by_status' => array_merge(array_fill_keys(self::STATUSES, 0), $by),
            'joined_later' => array_sum(array_column($members, 'joined_later')),
            'members' => $members,
        ];
    }

    // ── internal ─────────────────────────────────────────────────────────────────────────────────────────────

    /** Makes the invite code, mails the friend, the invite waits (status waiting, from create() or markPaid()) */
    private function send(string $id): void
    {
        $row = $this->row($id);
        $c = $this->campaigns->row((string) $row['campaign_id']);
        $until = CampaignStore::waitUntil($c, $this->clock->time());
        $code = $this->db->transaction(function () use ($id, $until) {
            do { $code = Ids::nanoid(8); } while ($this->db->value('SELECT 1 FROM qr WHERE code = ?', [$code]) !== null);
            $local = $this->clock->local();
            // a promo code (no own code type): reads like promo; it ends with the invite until the friend joins (then deliverTo)
            $this->db->run('INSERT INTO qr (id, code, redirect_to, type, email, created_at, claimed_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)',
                [Ids::uuid(), $code, rtrim($this->settings->get('APP_URL'), '/') . '/', 'promo', '', $local, $local, date('Y-m-d H:i:s', (int) strtotime($until . ' UTC'))]);
            $this->db->run('UPDATE campaign_invites SET invite_code = ?, sent_at = ?, expires_at = ? WHERE id = ?', [$code, $this->clock->utc(), $until, $id]);
            return $code;
        });
        $row = $this->row($id);
        $locale = (string) $row['locale'];
        $inviter = $this->users->byId((string) $row['inviter_id']);
        try {
            $this->mailer->send(MailTemplates::INVITE_RECEIVED, (string) $row['email'], $locale, [
                'from' => $inviter !== null ? (string) $inviter['email'] : '',
                'item' => $this->itemName($row, $locale),
                'code' => $code,
                'until' => self::date($until, $locale),
                'physical' => (int) $row['physical'] === 1,
                'url' => rtrim($this->settings->get('APP_URL'), '/') . '/?code=' . rawurlencode($code),
            ]);
        } catch (MailFailed) {
            // logged (admin → Emails); the invite stands – the member sees it waiting and can call it back
        }
    }

    /**
     * Inside a transaction: ends an open invite – the member's own item back, the invite code revoked (recall) or left to
     * run out (it ends with the invite).
     * @param array<string, mixed> $row locked
     * @return array<string, mixed> the row with its new status
     */
    private function end(array $row, string $status): array
    {
        $now = $this->clock->utc();
        if ($row['gift'] === 'own') {
            $this->inventory->add((string) $row['inviter_id'], (string) $row['item_id'], $row['variant'] !== null ? (string) $row['variant'] : null, (int) $row['qty'], 'gift');
        }
        if ($row['invite_code'] !== null && $status === 'recalled') {
            $this->db->run('UPDATE qr SET revoked_at = ?, revoked_reason = ? WHERE code = ?', [$this->clock->local(), 'invite recalled', $row['invite_code']]);
        }
        $this->db->run('UPDATE campaign_invites SET status = ?, ended_at = ? WHERE id = ?', [$status, $now, $row['id']]);
        return ['status' => $status, 'ended_at' => $now] + $row;
    }

    /** @param array<string, mixed> $row */
    private function mailReturned(array $row): void
    {
        $inviter = $this->users->byId((string) $row['inviter_id']);
        if ($inviter === null) return;
        $locale = MailTemplates::locale($inviter['locale'] ?? null);
        try {
            $this->mailer->send(MailTemplates::INVITE_RETURNED, (string) $inviter['email'], $locale, [
                'friend' => (string) $row['email'],
                'item' => $this->itemName($row, $locale),
                'unpaid' => $row['status'] === 'unpaid',
                'back' => $row['gift'] === 'own',
                'url' => rtrim($this->settings->get('APP_URL'), '/') . '/account',
            ]);
        } catch (MailFailed) {
            // logged; the member sees it in their invites
        }
    }

    /** @return array<string, mixed> */
    private function row(string $id): array
    {
        return $this->db->one('SELECT * FROM campaign_invites WHERE id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'This invite does not exist.');
    }

    /** @return array<string, mixed> */
    private function adminRow(string $id): array
    {
        return $this->db->one('SELECT i.*, u.email AS inviter_email, j.email AS joined_email FROM campaign_invites i LEFT JOIN users u ON u.id = i.inviter_id
            LEFT JOIN users j ON j.id = i.joined_user_id WHERE i.id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'This invite does not exist.');
    }

    /** @param array<string, mixed> $row */
    private function itemName(array $row, string $locale): string
    {
        return $this->transfers->itemName((string) $row['item_id'], $row['variant'] !== null ? (string) $row['variant'] : null, $locale);
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function memberJson(array $r, string $locale): array
    {
        return [
            'id' => (string) $r['id'],
            'email' => (string) $r['email'],
            'status' => (string) $r['status'],
            'gift' => (string) $r['gift'],
            'item_id' => (string) $r['item_id'],
            'variant' => $r['variant'],
            'item' => $this->itemName($r, $locale),
            'qty' => (int) $r['qty'],
            'physical' => (int) $r['physical'] === 1,
            'price_cents' => $r['price_cents'] !== null ? (int) $r['price_cents'] : null,
            'currency' => $r['currency'],
            'created_at' => Time::iso($r['created_at']),
            'payment_due_at' => Time::iso($r['payment_due_at']),
            'expires_at' => Time::iso($r['expires_at']),
            'joined_at' => Time::iso($r['joined_at']),
            'ended_at' => Time::iso($r['ended_at']),
            'shipped' => $r['shipped_at'] !== null,
        ];
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function adminJson(array $r): array
    {
        return $this->memberJson($r, 'en') + [
            'campaign_id' => (string) $r['campaign_id'],
            'inviter' => ['id' => (string) $r['inviter_id'], 'email' => $r['inviter_email'] ?? null],
            'joined' => $r['joined_user_id'] !== null ? ['id' => (string) $r['joined_user_id'], 'email' => $r['joined_email'] ?? null] : null,
            'invite_code' => $r['invite_code'],
            'paid_at' => Time::iso($r['paid_at']),
            'sent_at' => Time::iso($r['sent_at']),
            'ship' => $r['ship_given_at'] !== null ? ['name' => $r['ship_name'], 'address' => $r['ship_address'], 'country' => $r['ship_country'], 'given_at' => Time::iso($r['ship_given_at'])] : null,
            'shipped_at' => Time::iso($r['shipped_at']),
            'ship_note' => $r['ship_note'],
        ];
    }

    private static function price(int $cents, string $currency): string
    {
        return number_format($cents / 100, 2, ',', '.') . ' ' . $currency;
    }

    /** a UTC datetime as a date in the mail's language */
    private static function date(string $utc, string $locale): string
    {
        $time = (int) strtotime($utc . ' UTC');
        return $locale === 'de' ? gmdate('j.n.Y', $time) : gmdate('j M Y', $time);
    }
}
