<?php
declare(strict_types=1);

namespace Scutoons\Domain\Campaigns;

use PDOException;
use Scutoons\Domain\Accounts\ShippingAddress;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\Newsletter\NewsletterSignin;
use Scutoons\Domain\Ownership\OwnershipTransfers;
use Scutoons\Domain\Secrets;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;

/**
 * Giveaway campaigns – "Verschenken" (#66, agents/research/CAMPAIGN-GIFTS.md): SCUTOONS gives the campaign's item to
 * the recipients of a newsletter; they claim it ("Abholen") while the campaign runs.
 *   - the newsletter's `{gift}` paragraph = the recipient's button "Ein Geschenk wartet auf dich" (buttonFor): the
 *     sign-in link of NewsletterSignin (#65) + `&gift=<token>`. Members sign in, QR owners sign up with their product
 *     code, recipients who gave nothing but their email sign up with a personal promo code made here (campaign_codes:
 *     one per campaign + address, it signs up only that address – CodeService – and works until the campaign ends;
 *     at signup it gets the campaign's reading rule, like an invite's code). Staff: `?gift=` alone.
 *   - the token = base64url("<campaign id>|<email>") "." HMAC: only the account with that email can claim
 *   - claiming: once per address, while the campaign runs and its stock lasts; virtual → the inventory (the item's
 *     own per-account limit, `max`); physical → waits for the account's shipping address (`address`), then an admin
 *     sends it (`ship` → `shipped`); no address 30 days after the campaign's end → `lapsed`
 */
final class Giveaways
{
    public const LAPSE_DAYS = 30;
    private const TEXTS = [
        'de' => 'Ein Geschenk wartet auf dich',
        'en' => 'A gift is waiting for you',
    ];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Secrets $secrets,
        private readonly Catalog $catalog,
        private readonly InventoryService $inventory,
        private readonly OwnershipTransfers $transfers,
        private readonly CampaignStore $campaigns,
        private readonly NewsletterSignin $signin,
        private readonly ShippingAddress $shipping,
        private readonly Users $users,
        private readonly CodeService $codes,
    ) {
    }

    // ── The newsletter ───────────────────────────────────────────────────────────────────────────────────────

    /**
     * The `{gift}` button for one recipient of a newsletter (a mail_campaigns row); null = it has no gift
     * @param array<string, mixed> $newsletter
     * @return array{text: string, url: string}|null
     */
    public function buttonFor(array $newsletter, string $email, string $locale): ?array
    {
        $campaignId = $newsletter['gift_campaign_id'] ?? null;
        $campaign = $campaignId !== null ? $this->db->one("SELECT * FROM campaigns WHERE id = ? AND kind = 'giveaway'", [$campaignId]) : null;
        if ($campaign === null) return null;
        $locale = MailTemplates::locale($locale);
        $email = strtolower(trim($email));
        $signin = $this->signin->button($email, $locale)
            ?? ['url' => $this->signin->signupLink($email, $locale, $this->codeFor($campaign, $email))];
        $token = $this->token((string) $campaign['id'], $email);
        $url = str_contains($signin['url'], '?signin=') ? $signin['url'] . '&gift=' . $token : $this->signin->viewer($locale) . '?gift=' . $token;
        return ['text' => self::TEXTS[$locale], 'url' => $url];
    }

    /**
     * The button as the preview shows it (no login request; the link opens the viewer only)
     * @return array{text: string, url: string}
     */
    public static function sample(string $locale, string $appUrl): array
    {
        $locale = MailTemplates::locale($locale);
        return ['text' => self::TEXTS[$locale], 'url' => rtrim($appUrl, '/') . ($locale === 'en' ? '/en/' : '/')];
    }

    /**
     * Who of these addresses gets what (the newsletter editor says it, owner: "das das in der ui klar wird"):
     * members (sign in), QR owners without an account (sign up with their product code), email only (nothing given or
     * bought – a personal promo code)
     * @param list<string> $emails
     * @return array{members: int, product_code: int, email_only: int}
     */
    public function reach(array $emails): array
    {
        $out = ['members' => 0, 'product_code' => 0, 'email_only' => 0];
        foreach ($emails as $email) {
            $email = strtolower(trim($email));
            if ($this->users->byEmail($email) !== null) $out['members']++;
            elseif ($this->codes->legacySignupCode($email) !== null) $out['product_code']++;
            else $out['email_only']++;
        }
        return $out;
    }

    /**
     * Inside the signup transaction (LoginService::confirm): an account made with a giveaway's promo code gets the
     * campaign's reading rule on that code – keep → it never ends; until → the campaign's date
     * @param array<string, mixed> $user the new users row
     */
    public function joined(array $user, ?string $signupCode): void
    {
        if ($signupCode === null) return;
        try {
            $c = $this->db->one('SELECT c.access, c.access_until FROM campaign_codes cc JOIN campaigns c ON c.id = cc.campaign_id WHERE cc.code = ? AND cc.email = ?',
                [$signupCode, strtolower((string) $user['email'])]);
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            return;
        }
        if ($c === null) return;
        // qr times are local
        $until = $c['access'] === 'until' && $c['access_until'] !== null ? date('Y-m-d H:i:s', (int) strtotime($c['access_until'] . ' UTC')) : null;
        $this->db->run('UPDATE qr SET expires_at = ? WHERE code = ?', [$until, $signupCode]);
    }

    // ── The reader ───────────────────────────────────────────────────────────────────────────────────────────

    /**
     * GET /api/gift?t= – what the link holds and where it stands: state open | claimed (this address has it) |
     * not_started | ended | sold_out | other_account (signed in with another address). 404 invalid_token.
     * @param array<string, mixed>|null $user
     * @return array<string, mixed>
     */
    public function info(mixed $token, ?array $user, string $locale): array
    {
        [$campaign, $email] = $this->resolve($token);
        $claim = $this->claimOf((string) $campaign['id'], $email);
        $left = $this->campaigns->stockLeft($campaign);
        $state = match (true) {
            $claim !== null => 'claimed',
            $user !== null && strtolower((string) $user['email']) !== $email => 'other_account',
            $this->window($campaign) !== 'open' => $this->window($campaign),
            $left === 0 => 'sold_out',
            default => 'open',
        };
        $item = $this->catalog->item((string) $campaign['item_id']);
        $variant = $campaign['variant'] !== null ? (string) $campaign['variant'] : null;
        return [
            'state' => $state,
            'item' => [
                'id' => (string) $campaign['item_id'],
                'variant' => $variant,
                'name' => $this->transfers->itemName((string) $campaign['item_id'], $variant, MailTemplates::locale($locale)),
                'physical' => $item !== null && CampaignStore::isPhysical($item),
            ],
            'starts_at' => Time::iso($campaign['starts_at']),
            'until' => Time::iso($campaign['ends_at']),
            'left' => $left,
            'claim' => $claim !== null ? ['status' => (string) $claim['status'], 'created_at' => Time::iso($claim['created_at'])] : null,
        ];
    }

    /**
     * POST /api/gift { t } – the signed-in reader claims it. 404 invalid_token · 403 other_account · 409 already_claimed /
     * max_reached (the item's per-account limit) · 410 not_started / ended / sold_out.
     * @param array<string, mixed> $user
     * @return array<string, mixed> info()
     */
    public function claim(array $user, mixed $token, string $locale): array
    {
        [$campaign, $email] = $this->resolve($token);
        if (strtolower((string) $user['email']) !== $email) {
            throw new ApiError(403, 'other_account', 'This gift is for another email address – sign in with the one it was sent to.');
        }
        $userId = (string) $user['id'];
        $this->db->transaction(function () use ($campaign, $email, $userId) {
            // locked: the stock is counted by one claim at a time
            $c = $this->db->one('SELECT * FROM campaigns WHERE id = ? FOR UPDATE', [(string) $campaign['id']]) ?? $campaign;
            if ($this->claimOf((string) $c['id'], $email) !== null) throw new ApiError(409, 'already_claimed', 'You have this gift already.');
            $window = $this->window($c);
            if ($window === 'not_started') throw new ApiError(410, 'not_started', 'This gift can be claimed from ' . Time::iso($c['starts_at']) . '.');
            if ($window === 'ended') throw new ApiError(410, 'ended', 'The time to claim this gift is over.');
            if ($this->campaigns->stockLeft($c) === 0) throw new ApiError(410, 'sold_out', 'All of these gifts are given away.');
            $itemId = (string) $c['item_id'];
            $variant = $c['variant'] !== null ? (string) $c['variant'] : null;
            $item = $this->catalog->item($itemId) ?? throw new ApiError(404, 'invalid_token', 'This gift does not exist any more.');
            $ship = null;
            if (CampaignStore::isPhysical($item)) {
                $ship = $this->shipping->forCountries($userId, self::countries($c));
                $status = $ship !== null ? 'ship' : 'address';
            } else {
                if ($this->inventory->room($userId, $item) < 1) throw new ApiError(409, 'max_reached', 'You have as many of these as one account can have.', ['max' => InventoryService::maxOf($item)]);
                $this->inventory->add($userId, $itemId, $variant, 1, 'gift');
                $status = 'claimed';
            }
            $this->db->run('INSERT INTO campaign_claims (campaign_id, email, user_id, item_id, variant, status, ship_name, ship_address, ship_country, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', [(string) $c['id'], $email, $userId, $itemId, $variant, $status,
                $ship['name'] ?? null, $ship['address'] ?? null, $ship['country'] ?? null, $this->clock->utc()]);
        });
        return $this->info($token, $user, $locale);
    }

    /**
     * The claimed physical gifts of an account still waiting for its shipping address (GET /api/me/shipping)
     * @return list<array{id: int, item: string, countries: list<string>, until: ?string}>
     */
    public function waitingForAddress(string $userId, string $locale): array
    {
        try {
            $this->sweep();
            $rows = $this->db->all("SELECT g.*, c.countries, c.ends_at FROM campaign_claims g JOIN campaigns c ON c.id = g.campaign_id
                WHERE g.user_id = ? AND g.status = 'address' ORDER BY g.created_at", [$userId]);
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            return []; // before the 2026-10-20 migration
        }
        return array_map(fn(array $r) => [
            'id' => (int) $r['id'],
            'item' => $this->transfers->itemName((string) $r['item_id'], $r['variant'] !== null ? (string) $r['variant'] : null, MailTemplates::locale($locale)),
            'countries' => self::countries($r),
            'until' => Time::iso(gmdate('Y-m-d H:i:s', (int) strtotime($r['ends_at'] . ' UTC') + self::LAPSE_DAYS * 86400)),
        ], $rows);
    }

    /** After the account's address changed: its waiting physical gifts that ship to that country move on to `ship` */
    public function addressGiven(string $userId): int
    {
        $moved = 0;
        foreach ($this->db->all("SELECT g.id, c.countries FROM campaign_claims g JOIN campaigns c ON c.id = g.campaign_id
            WHERE g.user_id = ? AND g.status = 'address'", [$userId]) as $r) {
            $ship = $this->shipping->forCountries($userId, self::countries($r));
            if ($ship === null) continue;
            $this->db->run("UPDATE campaign_claims SET status = 'ship', ship_name = ?, ship_address = ?, ship_country = ? WHERE id = ? AND status = 'address'",
                [$ship['name'], $ship['address'], $ship['country'], (int) $r['id']]);
            $moved++;
        }
        return $moved;
    }

    /** Physical gifts still without an address 30 days after their campaign's end → lapsed. @return int how many */
    public function sweep(): int
    {
        $before = gmdate('Y-m-d H:i:s', $this->clock->time() - self::LAPSE_DAYS * 86400);
        return $this->db->run("UPDATE campaign_claims g JOIN campaigns c ON c.id = g.campaign_id SET g.status = 'lapsed'
            WHERE g.status = 'address' AND c.ends_at <= ?", [$before])->rowCount();
    }

    // ── Admin ────────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * GET /admin/campaigns/{id}/claims: the claims of a giveaway – counts by status, the physical ones in full
     * @return array{counts: array<string, int>, physical: list<array<string, mixed>>}
     */
    public function forCampaign(string $campaignId): array
    {
        $this->campaigns->row($campaignId);
        $this->sweep();
        $rows = $this->db->all("SELECT * FROM campaign_claims WHERE campaign_id = ? ORDER BY (status = 'ship') DESC, (status = 'address') DESC, created_at", [$campaignId]);
        $counts = ['claimed' => 0, 'address' => 0, 'ship' => 0, 'shipped' => 0, 'lapsed' => 0];
        foreach ($rows as $r) $counts[(string) $r['status']] = ($counts[(string) $r['status']] ?? 0) + 1;
        return [
            'counts' => $counts,
            'physical' => array_values(array_map(static fn(array $r) => [
                'id' => (int) $r['id'],
                'email' => (string) $r['email'],
                'status' => (string) $r['status'],
                'ship_name' => $r['ship_name'],
                'ship_address' => $r['ship_address'],
                'ship_country' => $r['ship_country'],
                'created_at' => Time::iso($r['created_at']),
                'shipped_at' => Time::iso($r['shipped_at']),
            ], array_filter($rows, static fn(array $r) => $r['status'] !== 'claimed'))),
        ];
    }

    /**
     * POST /admin/campaigns/claims/{id}/shipped – 404 not_found · 409 not_ready (no address yet / sent already)
     * @return string the campaign id
     */
    public function markShipped(int $claimId): string
    {
        $row = $this->db->one('SELECT campaign_id, status FROM campaign_claims WHERE id = ?', [$claimId]) ?? throw new ApiError(404, 'not_found', 'This gift does not exist.');
        $done = $this->db->run("UPDATE campaign_claims SET status = 'shipped', shipped_at = ? WHERE id = ? AND status = 'ship'", [$this->clock->utc(), $claimId]);
        if ($done->rowCount() !== 1) throw new ApiError(409, 'not_ready', 'Only a gift with a shipping address that was not sent yet can be marked sent.', ['status' => $row['status']]);
        return (string) $row['campaign_id'];
    }

    // ── Tokens, codes, rules ─────────────────────────────────────────────────────────────────────────────────

    public function token(string $campaignId, string $email): string
    {
        $payload = $campaignId . '|' . strtolower(trim($email));
        return self::b64($payload) . '.' . self::b64($this->mac($payload));
    }

    /**
     * The personal promo code of an email-only recipient (made once per campaign + address; ends with the campaign
     * until the account is made – then joined() sets the reading rule)
     * @param array<string, mixed> $campaign
     */
    private function codeFor(array $campaign, string $email): string
    {
        return $this->db->transaction(function () use ($campaign, $email): string {
            $code = $this->db->value('SELECT code FROM campaign_codes WHERE campaign_id = ? AND email = ? FOR UPDATE', [(string) $campaign['id'], $email]);
            if ($code !== null) return (string) $code;
            do { $code = Ids::nanoid(8); } while ($this->db->value('SELECT 1 FROM qr WHERE code = ?', [$code]) !== null);
            $local = $this->clock->local();
            $this->db->run('INSERT INTO qr (id, code, redirect_to, type, email, created_at, claimed_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)',
                [Ids::uuid(), $code, $this->signin->viewer('de'), 'promo', '', $local, $local, date('Y-m-d H:i:s', (int) strtotime($campaign['ends_at'] . ' UTC'))]);
            $this->db->run('INSERT INTO campaign_codes (campaign_id, email, code, created_at) VALUES (?, ?, ?, ?)', [(string) $campaign['id'], $email, $code, $this->clock->utc()]);
            return $code;
        });
    }

    /**
     * @return array{0: array<string, mixed>, 1: string} the giveaway campaign and the address
     */
    private function resolve(mixed $token): array
    {
        $invalid = new ApiError(404, 'invalid_token', 'This gift link is not valid.');
        if (!is_string($token) || strlen($token) > 800 || substr_count($token, '.') !== 1) throw $invalid;
        [$payload, $mac] = explode('.', $token);
        $payload = self::unb64($payload);
        $mac = self::unb64($mac);
        if ($payload === null || $mac === null || !hash_equals($this->mac($payload), $mac)) throw $invalid;
        $parts = explode('|', $payload, 2);
        if (count($parts) !== 2 || !preg_match('/^[0-9a-f-]{36}$/', $parts[0]) || $parts[1] === '') throw $invalid;
        $campaign = $this->db->one("SELECT * FROM campaigns WHERE id = ? AND kind = 'giveaway'", [$parts[0]]) ?? throw $invalid;
        return [$campaign, $parts[1]];
    }

    /** @return array<string, mixed>|null */
    private function claimOf(string $campaignId, string $email): ?array
    {
        return $this->db->one('SELECT * FROM campaign_claims WHERE campaign_id = ? AND email = ?', [$campaignId, $email]);
    }

    /**
     * @param array<string, mixed> $campaign
     * @return string open | not_started | ended
     */
    private function window(array $campaign): string
    {
        $now = $this->clock->utc();
        if ($campaign['status'] === 'stopped' || (string) $campaign['ends_at'] <= $now) return 'ended';
        if ($campaign['status'] !== 'started' || (string) $campaign['starts_at'] > $now) return 'not_started';
        return 'open';
    }

    /**
     * @param array<string, mixed> $row
     * @return list<string>
     */
    private static function countries(array $row): array
    {
        $list = (string) ($row['countries'] ?? '');
        return $list !== '' ? explode(',', $list) : [];
    }

    private function mac(string $payload): string
    {
        return substr(hash_hmac('sha256', $payload, $this->secrets->derivedKey('campaign-gift'), true), 0, 16);
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
