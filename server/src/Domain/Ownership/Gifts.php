<?php
declare(strict_types=1);

namespace Scutoons\Domain\Ownership;

use PDOException;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Campaigns\CampaignStore;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Mail\MailFailed;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\RateLimit\RateLimiter;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;

/**
 * Gifting between readers (agents/ITEMS.md "Ownership, claims, gifting"; owner 2026-10-02: "beautiful for game play" –
 * no money, no marketplace). POST /inventory/gift/ { entry_id | item_id (+ variant), qty?, to: email }.
 *
 * Rules (agents/research/second-hand-market.md, stage a): the item must be giftable (Catalog::giftable – not starters /
 * the basic dress / membership or access-only items), not worn (take it off first; a stackable keeps its worn piece),
 * not received in the last GIFT_COOLDOWN_HOURS (re-gift cooldown), at most GIFT_MAX_PER_DAY gifts per 24 h, the
 * recipient must have an account – no pending gifts; while a gift invite campaign runs for the giver, the 404
 * recipient_not_found carries `invite` (the offer, CampaignStore::offerFor) and the reader offers Invites instead
 * (agents/research/GIFT-INVITES.md) – never to yourself, and the recipient's collection limit holds (409 max_reached).
 * prepare / checkDailyLimit / lookup / takeable / takeOut are shared with Invites (an invite with the giver's own item).
 * A code-bound item (a physical product's twin) moves its whole code: the same transfer core as a claim, kind `gift`.
 * Every gift is in the transfer log; the recipient gets an email and a notice (GET /me/ `received`).
 */
final class Gifts
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Users $users,
        private readonly Catalog $catalog,
        private readonly InventoryService $inventory,
        private readonly OwnershipTransfers $transfers,
        private readonly RateLimiter $limits,
        private readonly Mailer $mailer,
        private readonly Settings $settings,
        private readonly CampaignStore $campaigns,
    ) {
    }

    /**
     * 400 invalid_email / self_gift / invalid_qty / invalid_entry · 404 recipient_not_found / not_found / unknown_item ·
     * 403 not_giftable · 409 worn / cooldown { retry_after } / max_reached { max } · 422 missing · 429 daily_limit /
     * rate_limited.
     * @param array<string, mixed> $giver users row
     * @param array<string, mixed> $body
     * @return array{item_id: string, variant: ?string, qty: int, code: ?string, to: string, transfer_id: int}
     */
    public function gift(array $giver, array $body): array
    {
        $this->transfers->requireTables();
        $giverId = (string) $giver['id'];
        $email = Users::normalizeEmail($body['to'] ?? null) ?? throw new ApiError(400, 'invalid_email', 'to: the email address of the reader you want to give it to.');
        if ($email === strtolower((string) $giver['email'])) throw new ApiError(400, 'self_gift', 'You cannot give yourself a gift.');
        ['entry' => $entry, 'item' => $item, 'code' => $code, 'qty' => $qty] = $this->prepare($giverId, $body);
        $this->checkDailyLimit($giverId);
        $this->lookup($giverId);
        $recipient = $this->users->byEmail($email);
        if ($recipient === null) {
            $offer = $this->campaigns->offerFor($giver);
            throw new ApiError(404, 'recipient_not_found', 'There is no SCUTOONS account with this email. Ask them to sign up first, then try again.', $offer !== null ? ['invite' => $offer] : []);
        }
        $recipientId = (string) $recipient['id'];

        $result = $this->db->transaction(function () use ($giverId, $recipient, $recipientId, $entry, $item, $code, $qty) {
            $entry = $this->takeable($giverId, $entry, $qty);

            if ($code !== null) {
                $moved = $this->transfers->moveCode($code, $recipient, 'gift', $giverId, null);
                return ['item_id' => (string) $entry['item_id'], 'variant' => $entry['variant'], 'qty' => 1, 'code' => $code, 'transfer_id' => $moved['transfer_id']];
            }
            if ($this->inventory->room($recipientId, $item) < $qty) {
                throw new ApiError(409, 'max_reached', 'They cannot hold more of this.', ['max' => InventoryService::maxOf($item)]);
            }
            $this->takeOut($giverId, $entry, $qty);
            $variant = $entry['variant'] !== null ? (string) $entry['variant'] : null;
            $this->inventory->add($recipientId, (string) $entry['item_id'], $variant, $qty, 'gift');
            $id = $this->transfers->record('gift', null, (string) $entry['item_id'], $variant, $qty, $giverId, $recipientId, $giverId, null, null);
            return ['item_id' => (string) $entry['item_id'], 'variant' => $variant, 'qty' => $qty, 'code' => null, 'transfer_id' => $id];
        });

        $locale = MailTemplates::locale($recipient['locale'] ?? null);
        try {
            $this->mailer->send(MailTemplates::GIFT_RECEIVED, (string) $recipient['email'], $locale, [
                'from' => (string) $giver['email'],
                'item' => $this->transfers->itemName($result['item_id'], $result['variant'], $locale),
                'qty' => $result['qty'],
                'url' => rtrim($this->settings->get('APP_URL'), '/') . '/account',
            ]);
        } catch (MailFailed) {
            // logged; the gift stands (the recipient also sees it on their next visit)
        }
        return $result + ['to' => (string) $recipient['email']];
    }

    /**
     * The giver's entry and item for a gift (outside the transaction): the entry, giftable, qty rules.
     * 400 invalid_qty / invalid_entry · 404 not_found / unknown_item · 403 not_giftable.
     * @param array<string, mixed> $body entry_id | item_id (+ variant), qty?
     * @return array{entry: array<string, mixed>, item: array<string, mixed>, code: ?string, qty: int}
     */
    public function prepare(string $giverId, array $body): array
    {
        $qty = $body['qty'] ?? 1;
        if (!is_int($qty) || $qty < 1 || $qty > InventoryService::MAX_PER_ACCOUNT) throw new ApiError(400, 'invalid_qty', 'qty: 1 or more.');
        $entry = $this->entry($giverId, $body);
        $item = $this->catalog->item((string) $entry['item_id']) ?? throw new ApiError(404, 'unknown_item', 'Unknown item.');
        if (!Catalog::giftable($item)) throw new ApiError(403, 'not_giftable', 'This item cannot be given away.');
        $code = $entry['code'] !== null ? (string) $entry['code'] : null;
        if ($code !== null && $qty !== 1) throw new ApiError(400, 'invalid_qty', 'A piece with a code is given on its own (qty 1).');
        if (empty($item['stackable']) && $qty !== 1) throw new ApiError(400, 'invalid_qty', 'This item is given one piece at a time.');
        return ['entry' => $entry, 'item' => $item, 'code' => $code, 'qty' => $qty];
    }

    /** GIFT_MAX_PER_DAY in 24 h – gifts and invites together (429 daily_limit) */
    public function checkDailyLimit(string $giverId): void
    {
        $window = 86400;
        $since = $this->clock->utc(-$window);
        $sent = (int) $this->db->value("SELECT COUNT(*) FROM item_transfers WHERE from_user_id = ? AND kind = 'gift' AND created_at > ?", [$giverId, $since]);
        try {
            $sent += (int) $this->db->value('SELECT COUNT(*) FROM campaign_invites WHERE inviter_id = ? AND created_at > ?', [$giverId, $since]);
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
        }
        $perDay = $this->settings->int('GIFT_MAX_PER_DAY');
        if ($sent >= $perDay) throw new ApiError(429, 'daily_limit', 'Enough gifts for today.', ['per_day' => $perDay, 'retry_after' => $window]);
    }

    /** Every recipient lookup counts (also unknown addresses): probing which emails have an account stays slow (429 rate_limited) */
    public function lookup(string $giverId): void
    {
        $this->limits->guard("giftlookup:$giverId", $this->settings->int('GIFT_LOOKUPS_PER_DAY'), 86400, 'Too many gifts tried today – try again tomorrow.');
        $this->limits->hit("giftlookup:$giverId");
    }

    /**
     * Inside the transaction: the entry read again (locked), not worn, enough, not received too recently.
     * 404 not_found · 409 worn / cooldown · 422 missing.
     * @param array<string, mixed> $entry
     * @return array<string, mixed> the fresh row
     */
    public function takeable(string $giverId, array $entry, int $qty): array
    {
        $entry = $this->db->one('SELECT * FROM inventory WHERE id = ? AND user_id = ? AND qty > 0 FOR UPDATE', [$entry['id'], $giverId])
            ?? throw new ApiError(404, 'not_found', 'This item is not in your inventory.');
        $worn = $this->db->value('SELECT 1 FROM avatar_slots WHERE user_id = ? AND entry_id = ? LIMIT 1', [$giverId, $entry['id']]) !== null;
        $free = (int) $entry['qty'] - ($worn ? 1 : 0);
        if ($free < 1) throw new ApiError(409, 'worn', 'Take it off first.');
        if ($qty > (int) $entry['qty']) throw new ApiError(422, 'missing', 'You do not have that many.', ['have' => (int) $entry['qty']]);
        if ($qty > $free) throw new ApiError(409, 'worn', 'One of them is worn – take it off first.');
        $this->checkCooldown($giverId, $entry, $qty);
        return $entry;
    }

    /**
     * Inside the transaction: $qty pieces of a virtual entry leave the giver's inventory
     * @param array<string, mixed> $entry
     */
    public function takeOut(string $giverId, array $entry, int $qty): void
    {
        if ($qty >= (int) $entry['qty']) {
            $this->db->run('DELETE FROM inventory WHERE id = ?', [$entry['id']]);
            $this->db->run('DELETE FROM avatar_slots WHERE user_id = ? AND entry_id = ?', [$giverId, $entry['id']]);
        } else {
            $this->db->run('UPDATE inventory SET qty = qty - ?, updated_at = ? WHERE id = ?', [$qty, $this->clock->utc(), $entry['id']]);
        }
    }

    /**
     * The giver's entry: by entry_id, else the first entry of item_id (+ variant) – the oldest virtual one first
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    private function entry(string $userId, array $body): array
    {
        $entryId = $body['entry_id'] ?? null;
        if ($entryId !== null) {
            if (!is_string($entryId) && !is_int($entryId)) throw new ApiError(400, 'invalid_entry', 'entry_id: an inventory entry id.');
            return $this->db->one('SELECT * FROM inventory WHERE id = ? AND user_id = ? AND qty > 0', [(string) $entryId, $userId])
                ?? throw new ApiError(404, 'not_found', 'This item is not in your inventory.');
        }
        $itemId = is_string($body['item_id'] ?? null) ? $body['item_id'] : throw new ApiError(400, 'invalid_entry', 'entry_id or item_id is required.');
        $variant = is_string($body['variant'] ?? null) && $body['variant'] !== '' ? $body['variant'] : null;
        $row = $this->db->one('SELECT * FROM inventory WHERE user_id = ? AND item_id = ? AND ' . ($variant === null ? 'variant IS NULL' : 'variant = ?')
            . ' AND qty > 0 ORDER BY (code IS NOT NULL), acquired_at, id LIMIT 1', $variant === null ? [$userId, $itemId] : [$userId, $itemId, $variant]);
        return $row ?? throw new ApiError(404, 'not_found', 'This item is not in your inventory.');
    }

    /**
     * Re-gift cooldown: what arrived (gift, claim, admin move) in the last GIFT_COOLDOWN_HOURS can't be passed on yet.
     * A code: its last move into this account. Other items: the pieces of this item received in the window stay back.
     * @param array<string, mixed> $entry
     */
    private function checkCooldown(string $userId, array $entry, int $qty): void
    {
        $hours = $this->settings->int('GIFT_COOLDOWN_HOURS');
        if ($hours <= 0) return;
        $since = $this->clock->utc(-$hours * 3600);
        if ($entry['code'] !== null) {
            $at = $this->db->value('SELECT MAX(created_at) FROM item_transfers WHERE code = ? AND to_user_id = ? AND created_at > ?', [$entry['code'], $userId, $since]);
            if (is_string($at) && $at !== '') throw new ApiError(409, 'cooldown', 'You just got this – you can pass it on later.', ['retry_after' => self::retryAfter($at, $hours, $this->clock->time())]);
            return;
        }
        $received = $this->db->all('SELECT qty, created_at FROM item_transfers WHERE to_user_id = ? AND item_id = ? AND code IS NULL AND created_at > ? ORDER BY created_at',
            [$userId, $entry['item_id'], $since]);
        if (!$received) return;
        $recent = array_sum(array_map(fn(array $r) => (int) $r['qty'], $received));
        $available = $this->inventory->count($userId, (string) $entry['item_id']) - $recent;
        if ($qty > $available) {
            throw new ApiError(409, 'cooldown', 'You just got this – you can pass it on later.',
                ['retry_after' => self::retryAfter((string) $received[0]['created_at'], $hours, $this->clock->time()), 'available' => max(0, $available)]);
        }
    }

    private static function retryAfter(string $receivedAt, int $hours, int $now): int
    {
        return max(1, (int) strtotime($receivedAt . ' UTC') + $hours * 3600 - $now);
    }
}
