<?php
declare(strict_types=1);

namespace Scutoons\Domain\Embassy;

use DateTimeImmutable;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Products;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Throwable;

/**
 * Embassy (embassy.scutoons.com, agents/EMBASSY.md): the partner network. Organisations (venues, festivals, radios,
 * shops, agents, press …) with their people, one lead per organisation × track with stage / owner / next check date,
 * and the activity timeline. Logging a call or a mail moves the lead on (cadence defaults below), so the Today list
 * always says who to call next.
 *
 * Access: role `ambassador` (or `admin`) = everything; role `liaison` = read everything, edit organisations and people,
 * work the leads that are theirs or unowned. Only ambassadors import, delete and assign leads to somebody else.
 *
 * Stores (agents/EMBASSY.md "Stores"): an organisation of a STOCKIST_TYPES type (`store`, `record_shop`) can carry
 * SCUTOONS products (`carries` = product ids of the shop catalog) and be `listed` on the public "Where to buy" page
 * (Stockists, GET /api/stockists/) with `independent`, `address`, `opening` – the other fields stay internal.
 *
 * Mail (agents/EMBASSY.md "Automation"): EmbassyMail sends a person's mail and logs it here as `mail_out`, EmbassyInbox
 * logs matched replies as `mail_in` (logMail(): subject, addresses, Message-ID – migration 2026-10-04-ask-embassy-mail;
 * before it the activity is logged without them).
 */
final class Embassy
{
    public const LIAISON = 'liaison';
    public const AMBASSADOR = 'ambassador';

    public const TRACKS = ['live', 'vinyl', 'booking', 'promo'];
    public const STAGES = ['research', 'ready', 'contacted', 'in_conversation', 'won', 'not_now', 'no', 'dormant'];
    public const TYPES = [
        'venue', 'festival', 'theatre', 'art_space', 'residency', 'radio', 'promoter', 'agency', 'distributor',
        'store', 'record_shop', 'label', 'blog', 'magazine', 'podcast', 'journalist', 'playlist', 'newsletter', 'ad_channel', 'other',
    ];
    /**
     * Types that sell to readers and so can carry products and be listed on the public "Where to buy" page: a record shop
     * is a store that specialises in records (the Vinyl track's targets, many came with the Top List import); both sell
     * over the counter. Distributors and labels sell to shops, not to readers, so they are not stockists.
     */
    public const STOCKIST_TYPES = ['store', 'record_shop'];
    public const KINDS = ['mail_out', 'mail_in', 'call', 'dm', 'meeting', 'note'];
    /** "answered before, quiet now": days without any entry (list filter `quiet`) */
    public const QUIET_DAYS = 90;
    public const OUTCOMES = ['reached', 'voicemail', 'no_answer', 'call_back', 'declined'];
    public const TIERS = ['A', 'B', 'C'];

    /** Cadence defaults in days (agents/EMBASSY.md "Communication strategy") */
    public const CALL_AFTER_MAIL = 6;
    public const RETRY_CALL = 3;
    public const QUIET_IN_CONVERSATION = 10;

    private const ORG_TEXT = ['name' => 255, 'city' => 255, 'website' => 1024, 'email' => 255, 'phone' => 64, 'source' => 255, 'notes' => 20000,
        'address' => 255, 'opening' => 255];
    private const PERSON_TEXT = ['name' => 255, 'role' => 255, 'email' => 255, 'phone' => 64, 'link' => 1024, 'language' => 16, 'notes' => 20000];
    private const MAX_IMPORT = 2000;
    /** The mail columns of embassy_activities exist (migration 2026-10-04-ask-embassy-mail) – null = not checked yet */
    private ?bool $mailColumns = null;

    public function __construct(
        private readonly Database $db,
        private readonly Roles $roles,
        private readonly Clock $clock,
        private readonly Products $products,
    ) {
    }

    /** @return list<string> the product ids a store may carry (the shop catalog) */
    public function productIds(): array
    {
        $ids = [];
        foreach ((array) ($this->products->catalog()['products'] ?? []) as $product) {
            if (is_array($product) && is_string($product['id'] ?? null)) $ids[] = $product['id'];
        }
        return $ids;
    }

    // ── Access ────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * The signed-in user's Embassy access: `ambassador` (role ambassador or admin) or `liaison`; 401 / 403 otherwise.
     * @param array<string, mixed>|null $user
     */
    public function access(?array $user): string
    {
        if ($user === null) throw new ApiError(401, 'unauthorized', 'Not signed in.');
        if ($this->roles->has($user, self::AMBASSADOR) || $this->roles->has($user, Roles::ADMIN)) return self::AMBASSADOR;
        if ($this->roles->has($user, self::LIAISON)) return self::LIAISON;
        throw new ApiError(403, 'forbidden', 'This needs the liaison or ambassador role.', ['role' => self::LIAISON]);
    }

    /** @param array<string, mixed> $user */
    private function requireAmbassador(array $user): void
    {
        if ($this->access($user) !== self::AMBASSADOR) {
            throw new ApiError(403, 'forbidden', 'Only ambassadors may do this.', ['role' => self::AMBASSADOR]);
        }
    }

    /** @return list<array{id: string, email: string, access: string}> everyone who may use the Embassy */
    public function team(): array
    {
        $rows = $this->db->all(
            "SELECT u.id, u.email, MAX(CASE WHEN r.name IN ('ambassador', 'admin') THEN 1 ELSE 0 END) AS amb
             FROM users u JOIN user_roles ur ON ur.user_id = u.id JOIN roles r ON r.id = ur.role_id
             WHERE r.name IN ('liaison', 'ambassador', 'admin') GROUP BY u.id, u.email ORDER BY u.email"
        );
        return array_map(fn(array $r) => ['id' => (string) $r['id'], 'email' => (string) $r['email'],
            'access' => (int) $r['amb'] === 1 ? self::AMBASSADOR : self::LIAISON], $rows);
    }

    // ── Organisations ─────────────────────────────────────────────────────────────────────────────────────

    /**
     * @param array<string, mixed> $filters q, type, country, track, stage, tier, owner (me | none | user id), due (1),
     *                                      quiet (days, "1" = QUIET_DAYS), sort (name | next), limit, offset
     * @param array<string, mixed> $user
     * @return array{items: list<array<string, mixed>>, total: int}
     */
    public function list(array $filters, array $user): array
    {
        $where = [];
        $params = [];
        $q = trim((string) ($filters['q'] ?? ''));
        if ($q !== '') {
            $where[] = "(o.name LIKE ? ESCAPE '!' OR o.city LIKE ? ESCAPE '!' OR o.tags LIKE ? ESCAPE '!')";
            $like = Database::likeContains($q);
            array_push($params, $like, $like, $like);
        }
        foreach (['type' => 'o.type', 'country' => 'o.country', 'tier' => 'o.tier'] as $key => $column) {
            $value = trim((string) ($filters[$key] ?? ''));
            if ($value === '') continue;
            $where[] = "$column = ?";
            $params[] = $key === 'type' ? $value : strtoupper($value);
        }
        $leadWhere = [];
        $leadParams = [];
        foreach (['track', 'stage'] as $key) {
            $value = trim((string) ($filters[$key] ?? ''));
            if ($value === '') continue;
            $leadWhere[] = "l.$key = ?";
            $leadParams[] = $value;
        }
        $owner = trim((string) ($filters['owner'] ?? ''));
        if ($owner === 'none') {
            $leadWhere[] = 'l.owner_id IS NULL';
        } elseif ($owner !== '') {
            $leadWhere[] = 'l.owner_id = ?';
            $leadParams[] = $owner === 'me' ? (string) $user['id'] : $owner;
        }
        if (($filters['due'] ?? '') === '1' || ($filters['due'] ?? null) === true) {
            $leadWhere[] = 'l.next_at IS NOT NULL AND l.next_at <= ?';
            $leadParams[] = $this->todayDate();
        }
        // "answered before, quiet now" (quietCondition()), not opted out
        $quiet = (string) ($filters['quiet'] ?? '');
        if ($quiet !== '' && $quiet !== '0') {
            [$sql, $quietParams] = $this->quietCondition($quiet === '1' || $quiet === 'true' ? self::QUIET_DAYS : max(1, min(3650, (int) $quiet)));
            $leadWhere[] = $sql;
            array_push($leadParams, ...$quietParams);
            $where[] = 'o.do_not_contact = 0';
        }
        if ($leadWhere) {
            $where[] = 'EXISTS (SELECT 1 FROM embassy_leads l WHERE l.org_id = o.id AND ' . implode(' AND ', $leadWhere) . ')';
            array_push($params, ...$leadParams);
        }
        $sql = ' FROM embassy_orgs o' . ($where ? ' WHERE ' . implode(' AND ', $where) : '');
        $total = (int) $this->db->value('SELECT COUNT(*)' . $sql, $params);
        $order = ($filters['sort'] ?? '') === 'next'
            ? ' ORDER BY (SELECT MIN(l2.next_at) FROM embassy_leads l2 WHERE l2.org_id = o.id) IS NULL, (SELECT MIN(l2.next_at) FROM embassy_leads l2 WHERE l2.org_id = o.id), o.name'
            : ' ORDER BY o.name_key, o.city';
        $limit = max(1, min(500, (int) ($filters['limit'] ?? 100)));
        $offset = max(0, (int) ($filters['offset'] ?? 0));
        $rows = $this->db->all("SELECT o.*, (SELECT COUNT(*) FROM embassy_people p WHERE p.org_id = o.id) AS people_count$sql$order LIMIT $limit OFFSET $offset", $params);
        $leads = $this->leadsOf(array_map(fn(array $r) => (string) $r['id'], $rows));
        $items = array_map(fn(array $r) => $this->org($r) + ['people_count' => (int) $r['people_count'], 'leads' => $leads[(string) $r['id']] ?? []], $rows);
        return ['items' => $items, 'total' => $total];
    }

    /** @return array{org: array<string, mixed>, people: list<array<string, mixed>>, leads: list<array<string, mixed>>, activities: list<array<string, mixed>>} */
    public function get(string $id): array
    {
        $row = $this->db->one('SELECT * FROM embassy_orgs WHERE id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'No such organisation.');
        $people = array_map(fn(array $p) => $this->person($p), $this->db->all('SELECT * FROM embassy_people WHERE org_id = ? ORDER BY created_at, name', [$id]));
        $activities = array_map(fn(array $a) => $this->activity($a),
            $this->db->all('SELECT a.*, l.track FROM embassy_activities a LEFT JOIN embassy_leads l ON l.id = a.lead_id WHERE a.org_id = ? ORDER BY a.at DESC, a.id DESC LIMIT 500', [$id]));
        return ['org' => $this->org($row), 'people' => $people, 'leads' => $this->leadsOf([$id])[$id] ?? [], 'activities' => $activities];
    }

    /**
     * Creates an organisation (+ optional `people` and `leads`). Same normalized name in the same city/country →
     * 409 duplicate { id } unless `force`.
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array<string, mixed> like get()
     */
    public function create(array $data, array $user): array
    {
        $this->access($user);
        $fields = $this->orgFields($data, true);
        if (($data['force'] ?? false) !== true && ($existing = $this->findDuplicate($fields)) !== null) {
            throw new ApiError(409, 'duplicate', 'This organisation exists already.', ['id' => $existing]);
        }
        $id = $this->db->transaction(function () use ($fields, $data, $user): string {
            $id = $this->insertOrg($fields, (string) ($user['email'] ?? ''));
            foreach ($this->listOf($data['people'] ?? []) as $person) $this->insertPerson($id, $this->personFields($person, true));
            foreach ($this->listOf($data['leads'] ?? []) as $lead) $this->writeLead($id, $lead, $user);
            return $id;
        });
        return $this->get($id);
    }

    /**
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array<string, mixed> like get()
     */
    public function update(string $id, array $data, array $user): array
    {
        $this->access($user);
        $this->get($id);
        $fields = $this->orgFields($data, false);
        if ($fields) {
            $sets = implode(', ', array_map(fn(string $k) => "$k = ?", array_keys($fields)));
            $this->db->run("UPDATE embassy_orgs SET $sets, updated_at = ? WHERE id = ?", [...array_values($fields), $this->clock->utc(), $id]);
        }
        return $this->get($id);
    }

    /** @param array<string, mixed> $user */
    public function delete(string $id, array $user): void
    {
        $this->requireAmbassador($user);
        $this->get($id);
        $this->db->transaction(function () use ($id): void {
            // explicit order: children first
            $this->db->run('DELETE FROM embassy_activities WHERE org_id = ?', [$id]);
            $this->db->run('DELETE FROM embassy_leads WHERE org_id = ?', [$id]);
            $this->db->run('DELETE FROM embassy_people WHERE org_id = ?', [$id]);
            $this->db->run('DELETE FROM embassy_orgs WHERE id = ?', [$id]);
        });
    }

    // ── People ────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array<string, mixed> the person
     */
    public function addPerson(string $orgId, array $data, array $user): array
    {
        $this->access($user);
        $this->get($orgId);
        $id = $this->insertPerson($orgId, $this->personFields($data, true));
        $this->touch($orgId);
        return $this->person($this->db->one('SELECT * FROM embassy_people WHERE id = ?', [$id]) ?? []);
    }

    /**
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array<string, mixed> the person
     */
    public function updatePerson(string $id, array $data, array $user): array
    {
        $this->access($user);
        $row = $this->db->one('SELECT * FROM embassy_people WHERE id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'No such person.');
        $fields = $this->personFields($data, false);
        if ($fields) {
            $sets = implode(', ', array_map(fn(string $k) => "$k = ?", array_keys($fields)));
            $this->db->run("UPDATE embassy_people SET $sets, updated_at = ? WHERE id = ?", [...array_values($fields), $this->clock->utc(), $id]);
            $this->touch((string) $row['org_id']);
        }
        return $this->person($this->db->one('SELECT * FROM embassy_people WHERE id = ?', [$id]) ?? []);
    }

    /** @param array<string, mixed> $user */
    public function deletePerson(string $id, array $user): void
    {
        $this->requireAmbassador($user);
        $row = $this->db->one('SELECT org_id FROM embassy_people WHERE id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'No such person.');
        $this->db->run('UPDATE embassy_activities SET person_id = NULL WHERE person_id = ?', [$id]);
        $this->db->run('DELETE FROM embassy_people WHERE id = ?', [$id]);
        $this->touch((string) $row['org_id']);
    }

    // ── Leads ─────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * Creates or changes the lead of an organisation in a track: stage, owner_id (null = nobody), next_at (YYYY-MM-DD
     * or null). A new lead without owner_id belongs to whoever creates it.
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array<string, mixed> the lead
     */
    public function setLead(string $orgId, string $track, array $data, array $user): array
    {
        $this->access($user);
        $this->get($orgId);
        $id = $this->db->transaction(fn() => $this->writeLead($orgId, ['track' => $track] + $data, $user));
        $this->touch($orgId);
        return $this->lead($id);
    }

    /**
     * Logs a mail / call / DM / meeting / note. With a `track` it belongs to that lead and moves it on:
     *   mail_out           research/ready → contacted, next = call in CALL_AFTER_MAIL days
     *   mail_in, meeting   → in_conversation (from research … contacted), next = QUIET_IN_CONVERSATION days
     *   call reached       → in_conversation (from research … contacted), next = QUIET_IN_CONVERSATION days
     *   call voicemail / no_answer → next = RETRY_CALL days
     *   call call_back     → next = next_at (required)
     *   call declined      → stage no, no next date
     * `stage` / `next_at` in the body override the defaults. A note changes nothing unless they are given.
     * Anything but a note on an unowned lead makes the user its owner.
     * @param array<string, mixed> $data kind, track?, person_id?, outcome?, note?, stage?, next_at?
     * @param array<string, mixed> $user
     * @return array{activity: array<string, mixed>, lead: array<string, mixed>|null}
     */
    public function log(string $orgId, array $data, array $user): array
    {
        return $this->write($orgId, $data, $user, []);
    }

    /**
     * Logs a mail (kind mail_out / mail_in) like log(), with its mail fields. $user null = the inbox sync (no access
     * check, nobody becomes the owner, a missing lead is not created). Without the migration the mail fields are dropped.
     * @param array<string, mixed> $data kind, track?, person_id?, note?, stage?, next_at?
     * @param array<string, mixed>|null $user
     * @param array{subject?: ?string, mail_from?: ?string, mail_to?: ?string, message_id?: ?string, in_reply_to?: ?string, at?: string, by_email?: string} $mail
     * @return array{activity: array<string, mixed>, lead: array<string, mixed>|null}
     */
    public function logMail(string $orgId, array $data, ?array $user, array $mail): array
    {
        return $this->write($orgId, $data, $user, $mail);
    }

    /** Whether embassy_activities has the mail columns (migration 2026-10-04-ask-embassy-mail) */
    public function hasMailColumns(): bool
    {
        if ($this->mailColumns === null) {
            try {
                $this->db->value('SELECT subject, mail_from, mail_to, message_id, in_reply_to FROM embassy_activities WHERE 1 = 0');
                $this->mailColumns = true;
            } catch (Throwable) {
                $this->mailColumns = false;
            }
        }
        return $this->mailColumns;
    }

    /**
     * Checks before a mail goes out what log() would refuse afterwards: a known track, and a liaison works only their
     * own or unowned leads (403 not_your_lead)
     * @param array<string, mixed> $user
     */
    public function assertMayLog(string $orgId, ?string $track, array $user): void
    {
        $this->access($user);
        if ($track === null) return;
        if (!in_array($track, self::TRACKS, true)) throw $this->invalid('track', self::TRACKS);
        $lead = $this->db->one('SELECT * FROM embassy_leads WHERE org_id = ? AND track = ?', [$orgId, $track]);
        if ($lead !== null) $this->assertMayWork($lead, $user);
    }

    /** Opt-out ("stop" reply): do-not-contact on the person, or on the organisation when there is no person */
    public function optOut(string $orgId, ?string $personId): void
    {
        if ($personId !== null) {
            $this->db->run('UPDATE embassy_people SET do_not_contact = 1, updated_at = ? WHERE id = ? AND org_id = ?', [$this->clock->utc(), $personId, $orgId]);
        } else {
            $this->db->run('UPDATE embassy_orgs SET do_not_contact = 1 WHERE id = ?', [$orgId]);
        }
        $this->touch($orgId);
    }

    /** @return array<string, mixed>|null one activity row (with its lead's track) */
    public function activityRow(string $id): ?array
    {
        return $this->db->one('SELECT a.*, l.track FROM embassy_activities a LEFT JOIN embassy_leads l ON l.id = a.lead_id WHERE a.id = ?', [$id]);
    }

    /**
     * log() / logMail()
     * @param array<string, mixed> $data
     * @param array<string, mixed>|null $user
     * @param array<string, mixed> $mail
     * @return array{activity: array<string, mixed>, lead: array<string, mixed>|null}
     */
    private function write(string $orgId, array $data, ?array $user, array $mail): array
    {
        if ($user !== null) $this->access($user);
        $this->get($orgId);
        $kind = (string) ($data['kind'] ?? '');
        if (!in_array($kind, self::KINDS, true)) throw $this->invalid('kind', self::KINDS);
        $outcome = isset($data['outcome']) && $data['outcome'] !== '' ? (string) $data['outcome'] : null;
        if ($kind === 'call' && !in_array($outcome, self::OUTCOMES, true)) throw $this->invalid('outcome', self::OUTCOMES);
        if ($kind !== 'call') $outcome = null;
        $personId = isset($data['person_id']) && $data['person_id'] !== '' ? (string) $data['person_id'] : null;
        if ($personId !== null && $this->db->value('SELECT 1 FROM embassy_people WHERE id = ? AND org_id = ?', [$personId, $orgId]) === null) {
            throw new ApiError(400, 'invalid_person', 'This person is not part of the organisation.');
        }
        $explicitNext = array_key_exists('next_at', $data) ? $this->date($data['next_at']) : false;
        if ($outcome === 'call_back' && !is_string($explicitNext)) {
            throw new ApiError(400, 'next_at_required', 'A call back needs the date (next_at).');
        }
        $note = mb_substr(trim((string) ($data['note'] ?? '')), 0, 20000);
        $track = isset($data['track']) && $data['track'] !== '' ? (string) $data['track'] : null;

        $result = $this->db->transaction(function () use ($orgId, $kind, $outcome, $personId, $explicitNext, $note, $track, $data, $user, $mail): array {
            $leadId = null;
            $existing = $track !== null ? $this->db->one('SELECT * FROM embassy_leads WHERE org_id = ? AND track = ?', [$orgId, $track]) : null;
            // the inbox sync (no user) never creates a lead
            if ($track !== null && ($existing !== null || $user !== null)) {
                $leadId = $existing !== null ? (string) $existing['id'] : $this->writeLead($orgId, ['track' => $track], $user ?? []);
                $lead = $this->db->one('SELECT * FROM embassy_leads WHERE id = ?', [$leadId]) ?? [];
                if ($user !== null) $this->assertMayWork($lead, $user);
                [$stage, $next] = $this->cadence((string) $lead['stage'], $kind, $outcome, $lead['next_at'] !== null ? (string) $lead['next_at'] : null);
                if (isset($data['stage']) && $data['stage'] !== '') $stage = $this->stage($data['stage']);
                if ($explicitNext !== false) $next = $explicitNext;
                $contact = in_array($kind, ['mail_out', 'mail_in', 'dm', 'meeting'], true) || ($kind === 'call' && $outcome !== 'no_answer');
                // whoever works an unowned lead takes it (it shows up on their Today list from now on)
                $owner = $lead['owner_id'] ?? null;
                if ($owner === null && $kind !== 'note' && $user !== null) $owner = (string) $user['id'];
                $this->db->run('UPDATE embassy_leads SET stage = ?, next_at = ?, owner_id = ?, last_contact_at = ' . ($contact ? '?' : 'last_contact_at') . ', updated_at = ? WHERE id = ?',
                    $contact ? [$stage, $next, $owner, $this->clock->utc(), $this->clock->utc(), $leadId] : [$stage, $next, $owner, $this->clock->utc(), $leadId]);
            }
            $row = [
                'id' => Ids::uuid7(), 'org_id' => $orgId, 'lead_id' => $leadId, 'person_id' => $personId, 'kind' => $kind, 'outcome' => $outcome, 'note' => $note,
                'by_user_id' => $user !== null ? (string) $user['id'] : null,
                'by_email' => $user !== null ? (string) ($user['email'] ?? '') : mb_substr((string) ($mail['by_email'] ?? ''), 0, 255),
                'at' => (string) ($mail['at'] ?? $this->clock->utc()),
            ];
            if ($mail && $this->hasMailColumns()) {
                foreach (['subject' => 300, 'mail_from' => 254, 'mail_to' => 254, 'message_id' => 255, 'in_reply_to' => 255] as $key => $max) {
                    $value = isset($mail[$key]) && $mail[$key] !== '' ? mb_substr((string) $mail[$key], 0, $max) : null;
                    if ($value !== null) $row[$key] = $value;
                }
            }
            $this->db->run('INSERT INTO embassy_activities (' . implode(', ', array_keys($row)) . ') VALUES (' . implode(', ', array_fill(0, count($row), '?')) . ')', array_values($row));
            return [$row['id'], $leadId];
        });
        $this->touch($orgId);
        $activity = $this->activityRow($result[0]) ?? [];
        return ['activity' => $this->activity($activity), 'lead' => $result[1] !== null ? $this->lead($result[1]) : null];
    }

    /**
     * The due list: leads whose next check date is today or earlier (oldest first), for one owner ('me' = the user,
     * 'all' = everybody, 'none' = unowned), plus how many leads wait in research/ready without a date.
     * @param array<string, mixed> $user
     * @return array{today: string, due: list<array<string, mixed>>, unscheduled: int}
     */
    public function today(string $owner, array $user): array
    {
        $this->access($user);
        $today = $this->todayDate();
        [$ownerSql, $params] = match ($owner) {
            'all' => ['', []],
            'none' => [' AND l.owner_id IS NULL', []],
            default => [' AND l.owner_id = ?', [$owner === 'me' || $owner === '' ? (string) $user['id'] : $owner]],
        };
        $rows = $this->db->all("SELECT l.id FROM embassy_leads l JOIN embassy_orgs o ON o.id = l.org_id
            WHERE l.next_at IS NOT NULL AND l.next_at <= ? AND o.do_not_contact = 0$ownerSql ORDER BY l.next_at, o.name_key LIMIT 300", [$today, ...$params]);
        $due = [];
        foreach ($rows as $row) {
            $item = $this->item((string) $row['id']);
            $due[] = $item + ['overdue_days' => (int) (new DateTimeImmutable($today))->diff(new DateTimeImmutable((string) $item['lead']['next_at']))->days];
        }
        $unscheduled = (int) $this->db->value("SELECT COUNT(*) FROM embassy_leads l WHERE l.next_at IS NULL AND l.stage IN ('research', 'ready')$ownerSql", $params);
        return ['today' => $today, 'due' => $due, 'unscheduled' => $unscheduled];
    }

    /**
     * One lead as a work item (the Today list, the weekly portion – EmbassyPace): the lead, its organisation, the
     * people that may be contacted and the lead's last activity
     * @return array{lead: array<string, mixed>, org: array<string, mixed>, people: list<array<string, mixed>>, last_activity: array<string, mixed>|null}
     */
    public function item(string $leadId): array
    {
        $lead = $this->lead($leadId);
        $org = $this->org($this->db->one('SELECT * FROM embassy_orgs WHERE id = ?', [$lead['org_id']]) ?? []);
        $people = array_map(fn(array $p) => $this->person($p),
            $this->db->all('SELECT * FROM embassy_people WHERE org_id = ? AND do_not_contact = 0 ORDER BY created_at, name', [$lead['org_id']]));
        $last = $this->db->one('SELECT a.*, l.track FROM embassy_activities a LEFT JOIN embassy_leads l ON l.id = a.lead_id WHERE a.lead_id = ? ORDER BY a.at DESC, a.id DESC LIMIT 1', [$leadId]);
        return ['lead' => $lead, 'org' => $org, 'people' => $people, 'last_activity' => $last !== null ? $this->activity($last) : null];
    }

    /**
     * "Answered before, quiet now" (owner 2026-10-04: keep track of dormant contacts that were responsive in the past) as
     * a condition on the lead `l`: the lead (or the organisation, for entries without a track) had a reply, a meeting or
     * a reached call, nothing happened for $days days and no date is ahead – the people worth a call again. Not: "no".
     * (The organisation's do-not-contact is the caller's part.) List filter `quiet`, the weekly portion's `quiet` group.
     * @return array{0: string, 1: list<string>} SQL + its parameters
     */
    public function quietCondition(int $days): array
    {
        $mine = '(a.lead_id = l.id OR (a.lead_id IS NULL AND a.org_id = l.org_id))';
        return ["l.stage <> 'no' AND (l.next_at IS NULL OR l.next_at <= ?)"
            . " AND EXISTS (SELECT 1 FROM embassy_activities a WHERE $mine AND (a.kind IN ('mail_in', 'meeting') OR (a.kind = 'call' AND a.outcome = 'reached')))"
            . " AND NOT EXISTS (SELECT 1 FROM embassy_activities a WHERE $mine AND a.at >= ?)",
            [$this->todayDate(), $this->clock->utc(-$days * 86400)]];
    }

    // ── Import ────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * Bulk import (ambassador): `organisations` = list of create() bodies. An organisation with the same normalized
     * name in the same city/country is merged: empty fields filled, links/tags/territories united, new people (by
     * email, else by name) and new tracks added – nothing existing is overwritten.
     * @param array<string, mixed> $data organisations, dry_run?
     * @param array<string, mixed> $user
     * @return array{dry_run: bool, created: int, merged: int, people_added: int, leads_added: int, errors: list<array{index: int, name: string, error: string}>}
     */
    public function import(array $data, array $user): array
    {
        $this->requireAmbassador($user);
        $orgs = $this->listOf($data['organisations'] ?? null);
        if (!$orgs) throw new ApiError(400, 'invalid_body', 'Expected { organisations: [ … ] }.');
        if (count($orgs) > self::MAX_IMPORT) throw new ApiError(400, 'too_many', 'At most ' . self::MAX_IMPORT . ' organisations per import.');
        $dryRun = ($data['dry_run'] ?? false) === true;
        $report = ['dry_run' => $dryRun, 'created' => 0, 'merged' => 0, 'people_added' => 0, 'leads_added' => 0, 'errors' => []];
        $run = function () use ($orgs, $user, &$report): void {
            foreach ($orgs as $index => $item) {
                try {
                    $fields = $this->orgFields($item, true);
                    $id = $this->findDuplicate($fields);
                    if ($id === null) {
                        $id = $this->insertOrg($fields, (string) ($user['email'] ?? ''));
                        $report['created']++;
                    } else {
                        $this->mergeOrg($id, $fields);
                        $report['merged']++;
                    }
                    foreach ($this->listOf($item['people'] ?? []) as $person) {
                        $p = $this->personFields($person, true);
                        if ($p['name'] === '' && $p['email'] === '' && $p['phone'] === '') continue;
                        $known = $p['email'] !== ''
                            ? $this->db->value('SELECT 1 FROM embassy_people WHERE org_id = ? AND LOWER(email) = ?', [$id, strtolower($p['email'])])
                            : $this->db->value('SELECT 1 FROM embassy_people WHERE org_id = ? AND LOWER(name) = ?', [$id, mb_strtolower($p['name'])]);
                        if ($known !== null) continue;
                        $this->insertPerson($id, $p);
                        $report['people_added']++;
                    }
                    foreach ($this->listOf($item['leads'] ?? []) as $lead) {
                        $track = (string) ($lead['track'] ?? '');
                        if ($this->db->value('SELECT 1 FROM embassy_leads WHERE org_id = ? AND track = ?', [$id, $track]) !== null) continue;
                        $this->writeLead($id, $lead + ['owner_id' => null], $user);
                        $report['leads_added']++;
                    }
                } catch (ApiError $e) {
                    $report['errors'][] = ['index' => (int) $index, 'name' => (string) ($item['name'] ?? ''), 'error' => $e->getMessage()];
                }
            }
        };
        if ($dryRun) {
            // same code path, rolled back
            try {
                $this->db->transaction(function () use ($run): void {
                    $run();
                    throw new DryRun();
                });
            } catch (DryRun) {
            }
        } else {
            $this->db->transaction($run);
        }
        return $report;
    }

    // ── Internals ─────────────────────────────────────────────────────────────────────────────────────────

    /** Today's date in the API's time zone (YYYY-MM-DD) – next_at dates are local dates */
    public function todayDate(): string
    {
        return substr($this->clock->local(), 0, 10);
    }

    /**
     * Stage and next date after an activity (see log())
     * @return array{0: string, 1: ?string}
     */
    private function cadence(string $stage, string $kind, ?string $outcome, ?string $next): array
    {
        $early = in_array($stage, ['research', 'ready', 'contacted', 'not_now', 'dormant'], true);
        return match (true) {
            $kind === 'mail_out' => [in_array($stage, ['research', 'ready'], true) ? 'contacted' : $stage, $this->inDays($stage === 'in_conversation' ? self::QUIET_IN_CONVERSATION : self::CALL_AFTER_MAIL)],
            $kind === 'mail_in', $kind === 'meeting', $kind === 'dm' && !$early => [$early ? 'in_conversation' : $stage, $this->inDays(self::QUIET_IN_CONVERSATION)],
            $kind === 'call' && $outcome === 'reached' => [$early ? 'in_conversation' : $stage, $this->inDays(self::QUIET_IN_CONVERSATION)],
            $kind === 'call' && in_array($outcome, ['voicemail', 'no_answer'], true) => [$stage, $this->inDays(self::RETRY_CALL)],
            $kind === 'call' && $outcome === 'declined' => ['no', null],
            $kind === 'dm' => [in_array($stage, ['research', 'ready'], true) ? 'contacted' : $stage, $this->inDays(self::CALL_AFTER_MAIL)],
            default => [$stage, $next],
        };
    }

    /** Today + $days, moved off the weekend to Monday (follow-ups are calls in office hours) */
    private function inDays(int $days): string
    {
        $date = (new DateTimeImmutable($this->todayDate()))->modify("+$days days");
        $weekday = (int) $date->format('N');
        return ($weekday >= 6 ? $date->modify('+' . (8 - $weekday) . ' days') : $date)->format('Y-m-d');
    }

    /**
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return string the lead id
     */
    private function writeLead(string $orgId, array $data, array $user): string
    {
        $track = (string) ($data['track'] ?? '');
        if (!in_array($track, self::TRACKS, true)) throw $this->invalid('track', self::TRACKS);
        $ambassador = $this->access($user) === self::AMBASSADOR;
        $existing = $this->db->one('SELECT * FROM embassy_leads WHERE org_id = ? AND track = ?', [$orgId, $track]);
        $sets = [];
        if (isset($data['stage']) && $data['stage'] !== '') $sets['stage'] = $this->stage($data['stage']);
        if (array_key_exists('next_at', $data)) $sets['next_at'] = $this->date($data['next_at']);
        if (array_key_exists('owner_id', $data)) {
            $owner = $data['owner_id'] === null || $data['owner_id'] === '' ? null : (string) $data['owner_id'];
            if ($owner !== null && $owner !== (string) $user['id'] && !$ambassador) {
                throw new ApiError(403, 'forbidden', 'Only ambassadors assign leads to somebody else.', ['role' => self::AMBASSADOR]);
            }
            if ($owner !== null && $this->db->value('SELECT 1 FROM users WHERE id = ?', [$owner]) === null) {
                throw new ApiError(400, 'invalid_owner', 'Unknown user.');
            }
            $sets['owner_id'] = $owner;
        }
        $now = $this->clock->utc();
        if ($existing === null) {
            $id = Ids::uuid7();
            $this->db->run('INSERT INTO embassy_leads (id, org_id, track, stage, owner_id, next_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)', [
                $id, $orgId, $track, $sets['stage'] ?? 'research',
                array_key_exists('owner_id', $sets) ? $sets['owner_id'] : (string) $user['id'],
                $sets['next_at'] ?? null, $now, $now,
            ]);
            return $id;
        }
        $this->assertMayWork($existing, $user);
        if ($sets) {
            $sql = implode(', ', array_map(fn(string $k) => "$k = ?", array_keys($sets)));
            $this->db->run("UPDATE embassy_leads SET $sql, updated_at = ? WHERE id = ?", [...array_values($sets), $now, (string) $existing['id']]);
        }
        return (string) $existing['id'];
    }

    /**
     * Liaisons work their own and unowned leads only
     * @param array<string, mixed> $lead
     * @param array<string, mixed> $user
     */
    private function assertMayWork(array $lead, array $user): void
    {
        if ($this->access($user) === self::AMBASSADOR) return;
        $owner = $lead['owner_id'] ?? null;
        if ($owner !== null && (string) $owner !== (string) $user['id']) {
            throw new ApiError(403, 'not_your_lead', 'This lead belongs to somebody else – ask an ambassador to hand it over.');
        }
    }

    /**
     * @param array<string, mixed> $data
     * @return array<string, mixed> column → value (all columns when $create, else only the given ones)
     */
    private function orgFields(array $data, bool $create): array
    {
        $fields = [];
        foreach (self::ORG_TEXT as $key => $max) {
            if (!$create && !array_key_exists($key, $data)) continue;
            $value = mb_substr(trim((string) ($data[$key] ?? '')), 0, $max);
            if ($key === 'website') $value = self::url($value);
            if ($key === 'email' && $value !== '' && filter_var($value, FILTER_VALIDATE_EMAIL) === false) throw new ApiError(400, 'invalid_email', "Not an email address: $value", ['field' => 'email']);
            $fields[$key] = $value;
        }
        if (array_key_exists('name', $fields)) {
            if ($fields['name'] === '') throw new ApiError(400, 'name_required', 'The organisation needs a name.', ['field' => 'name']);
            $fields['name_key'] = self::nameKey((string) $fields['name']);
        }
        if ($create || array_key_exists('type', $data)) {
            $type = (string) ($data['type'] ?? 'other');
            if (!in_array($type, self::TYPES, true)) throw $this->invalid('type', self::TYPES);
            $fields['type'] = $type;
        }
        if ($create || array_key_exists('country', $data)) $fields['country'] = $this->country($data['country'] ?? '');
        if ($create || array_key_exists('tier', $data)) {
            $tier = strtoupper(trim((string) ($data['tier'] ?? '')));
            if ($tier !== '' && !in_array($tier, self::TIERS, true)) throw $this->invalid('tier', self::TIERS);
            $fields['tier'] = $tier === '' ? null : $tier;
        }
        foreach (['links', 'tags', 'territories'] as $key) {
            if (!$create && !array_key_exists($key, $data)) continue;
            $fields[$key] = json_encode($this->strings($data[$key] ?? [], $key === 'territories'), JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        }
        if ($create || array_key_exists('do_not_contact', $data)) $fields['do_not_contact'] = ($data['do_not_contact'] ?? false) ? 1 : 0;
        // stores (STOCKIST_TYPES): listed on the public page, small / independent vendor, the products they carry
        foreach (['listed', 'independent'] as $key) {
            if ($create || array_key_exists($key, $data)) $fields[$key] = ($data[$key] ?? false) ? 1 : 0;
        }
        if ($create || array_key_exists('carries', $data)) {
            $carries = $this->strings($data['carries'] ?? [], false);
            $known = $carries ? $this->productIds() : [];
            $unknown = array_values(array_diff($carries, $known));
            if ($unknown) {
                throw new ApiError(400, 'invalid_product', 'Unknown product: ' . implode(', ', $unknown), ['field' => 'carries', 'allowed' => $known]);
            }
            $fields['carries'] = json_encode($carries, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        }
        return $fields;
    }

    /**
     * @param array<string, mixed> $data
     * @return array<string, mixed>
     */
    private function personFields(array $data, bool $create): array
    {
        $fields = [];
        foreach (self::PERSON_TEXT as $key => $max) {
            if (!$create && !array_key_exists($key, $data)) continue;
            $value = mb_substr(trim((string) ($data[$key] ?? '')), 0, $max);
            if ($key === 'link') $value = self::url($value);
            if ($key === 'email' && $value !== '' && filter_var($value, FILTER_VALIDATE_EMAIL) === false) throw new ApiError(400, 'invalid_email', "Not an email address: $value", ['field' => 'email']);
            $fields[$key] = $value;
        }
        if ($create || array_key_exists('territories', $data)) {
            $fields['territories'] = json_encode($this->strings($data['territories'] ?? [], true));
        }
        if ($create || array_key_exists('do_not_contact', $data)) $fields['do_not_contact'] = ($data['do_not_contact'] ?? false) ? 1 : 0;
        return $fields;
    }

    /** @param array<string, mixed> $fields */
    private function insertOrg(array $fields, string $by): string
    {
        $id = Ids::uuid7();
        $now = $this->clock->utc();
        $all = $fields + ['id' => $id, 'created_at' => $now, 'updated_at' => $now, 'created_by' => $by !== '' ? $by : null];
        $columns = implode(', ', array_keys($all));
        $this->db->run("INSERT INTO embassy_orgs ($columns) VALUES (" . implode(', ', array_fill(0, count($all), '?')) . ')', array_values($all));
        return $id;
    }

    /** @param array<string, mixed> $fields */
    private function insertPerson(string $orgId, array $fields): string
    {
        $id = Ids::uuid7();
        $now = $this->clock->utc();
        $all = $fields + ['id' => $id, 'org_id' => $orgId, 'created_at' => $now, 'updated_at' => $now];
        $columns = implode(', ', array_keys($all));
        $this->db->run("INSERT INTO embassy_people ($columns) VALUES (" . implode(', ', array_fill(0, count($all), '?')) . ')', array_values($all));
        return $id;
    }

    /**
     * Fills empty fields and unites the lists, never overwrites
     * @param array<string, mixed> $fields
     */
    private function mergeOrg(string $id, array $fields): void
    {
        $row = $this->db->one('SELECT * FROM embassy_orgs WHERE id = ?', [$id]) ?? [];
        $sets = [];
        foreach (['city', 'country', 'website', 'email', 'phone', 'source', 'tier', 'address', 'opening'] as $key) {
            if (($row[$key] ?? '') === '' || $row[$key] === null) {
                if (($fields[$key] ?? '') !== '' && $fields[$key] !== null) $sets[$key] = $fields[$key];
            }
        }
        if (($row['type'] ?? 'other') === 'other' && ($fields['type'] ?? 'other') !== 'other') $sets['type'] = $fields['type'];
        if (($fields['notes'] ?? '') !== '' && !str_contains((string) $row['notes'], (string) $fields['notes'])) {
            $sets['notes'] = trim($row['notes'] . "\n\n" . $fields['notes']);
        }
        foreach (['links', 'tags', 'territories', 'carries'] as $key) {
            $merged =array_values(array_unique([...self::decode($row[$key] ?? '[]'), ...self::decode($fields[$key] ?? '[]')]));
            if ($merged !== self::decode($row[$key] ?? '[]')) $sets[$key] = json_encode($merged, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        }
        if (!$sets) return;
        $sql = implode(', ', array_map(fn(string $k) => "$k = ?", array_keys($sets)));
        $this->db->run("UPDATE embassy_orgs SET $sql, updated_at = ? WHERE id = ?", [...array_values($sets), $this->clock->utc(), $id]);
    }

    /**
     * Same normalized name and the same city (or country), or one side has neither
     * @param array<string, mixed> $fields
     */
    private function findDuplicate(array $fields): ?string
    {
        foreach ($this->db->all('SELECT id, city, country FROM embassy_orgs WHERE name_key = ?', [$fields['name_key']]) as $row) {
            $cityA = self::nameKey((string) $row['city']);
            $cityB = self::nameKey((string) ($fields['city'] ?? ''));
            if ($cityA !== '' && $cityB !== '') {
                if ($cityA === $cityB) return (string) $row['id'];
                continue;
            }
            $countryA = (string) $row['country'];
            $countryB = (string) ($fields['country'] ?? '');
            if ($countryA === '' || $countryB === '' || $countryA === $countryB) return (string) $row['id'];
        }
        return null;
    }

    private function touch(string $orgId): void
    {
        $this->db->run('UPDATE embassy_orgs SET updated_at = ? WHERE id = ?', [$this->clock->utc(), $orgId]);
    }

    /**
     * @param list<string> $orgIds
     * @return array<string, list<array<string, mixed>>> org id → leads (track order)
     */
    private function leadsOf(array $orgIds): array
    {
        if (!$orgIds) return [];
        $rows = $this->db->all('SELECT l.*, u.email AS owner_email FROM embassy_leads l LEFT JOIN users u ON u.id = l.owner_id WHERE l.org_id IN ('
            . implode(', ', array_fill(0, count($orgIds), '?')) . ')', $orgIds);
        usort($rows, fn(array $a, array $b) => array_search($a['track'], self::TRACKS, true) <=> array_search($b['track'], self::TRACKS, true));
        $out = [];
        foreach ($rows as $row) $out[(string) $row['org_id']][] = $this->leadRow($row);
        return $out;
    }

    /** @return array<string, mixed> */
    private function lead(string $id): array
    {
        $row = $this->db->one('SELECT l.*, u.email AS owner_email FROM embassy_leads l LEFT JOIN users u ON u.id = l.owner_id WHERE l.id = ?', [$id])
            ?? throw new ApiError(404, 'not_found', 'No such lead.');
        return $this->leadRow($row);
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function leadRow(array $r): array
    {
        return [
            'id' => (string) $r['id'], 'org_id' => (string) $r['org_id'], 'track' => (string) $r['track'], 'stage' => (string) $r['stage'],
            'owner_id' => $r['owner_id'] !== null ? (string) $r['owner_id'] : null, 'owner_email' => $r['owner_email'] !== null ? (string) $r['owner_email'] : null,
            'next_at' => $r['next_at'] !== null ? substr((string) $r['next_at'], 0, 10) : null,
            'last_contact_at' => $r['last_contact_at'] !== null ? (string) $r['last_contact_at'] : null, 'updated_at' => (string) $r['updated_at'],
        ];
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function org(array $r): array
    {
        return [
            'id' => (string) $r['id'], 'name' => (string) $r['name'], 'type' => (string) $r['type'], 'city' => (string) $r['city'],
            'country' => (string) $r['country'], 'website' => (string) $r['website'], 'email' => (string) $r['email'], 'phone' => (string) $r['phone'],
            'links' => self::decode($r['links'] ?? '[]'), 'tags' => self::decode($r['tags'] ?? '[]'), 'territories' => self::decode($r['territories'] ?? '[]'),
            'tier' => $r['tier'] !== null && $r['tier'] !== '' ? (string) $r['tier'] : null, 'source' => (string) $r['source'], 'notes' => (string) $r['notes'],
            'do_not_contact' => (int) $r['do_not_contact'] === 1, 'created_at' => (string) $r['created_at'], 'updated_at' => (string) $r['updated_at'],
            // stores (migration 2026-10-02-embassy-stores)
            'listed' => (int) ($r['listed'] ?? 0) === 1, 'independent' => (int) ($r['independent'] ?? 0) === 1,
            'address' => (string) ($r['address'] ?? ''), 'opening' => (string) ($r['opening'] ?? ''), 'carries' => self::decode($r['carries'] ?? '[]'),
        ];
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function person(array $r): array
    {
        return [
            'id' => (string) $r['id'], 'org_id' => (string) $r['org_id'], 'name' => (string) $r['name'], 'role' => (string) $r['role'],
            'email' => (string) $r['email'], 'phone' => (string) $r['phone'], 'link' => (string) $r['link'], 'language' => (string) $r['language'],
            'territories' => self::decode($r['territories'] ?? '[]'), 'notes' => (string) $r['notes'], 'do_not_contact' => (int) $r['do_not_contact'] === 1,
        ];
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function activity(array $r): array
    {
        return [
            'id' => (string) $r['id'], 'org_id' => (string) $r['org_id'], 'lead_id' => $r['lead_id'] !== null ? (string) $r['lead_id'] : null,
            'track' => isset($r['track']) ? (string) $r['track'] : null, 'person_id' => $r['person_id'] !== null ? (string) $r['person_id'] : null,
            'kind' => (string) $r['kind'], 'outcome' => $r['outcome'] !== null ? (string) $r['outcome'] : null, 'note' => (string) $r['note'],
            'by_email' => (string) $r['by_email'], 'at' => (string) $r['at'],
            // mails (migration 2026-10-04-ask-embassy-mail)
            'subject' => isset($r['subject']) ? (string) $r['subject'] : null, 'mail_from' => isset($r['mail_from']) ? (string) $r['mail_from'] : null,
            'mail_to' => isset($r['mail_to']) ? (string) $r['mail_to'] : null, 'message_id' => isset($r['message_id']) ? (string) $r['message_id'] : null,
        ];
    }

    private function stage(mixed $value): string
    {
        $stage = (string) $value;
        if (!in_array($stage, self::STAGES, true)) throw $this->invalid('stage', self::STAGES);
        return $stage;
    }

    /** YYYY-MM-DD or null */
    private function date(mixed $value): ?string
    {
        if ($value === null || $value === '') return null;
        $text = (string) $value;
        if (!preg_match('/^(\d{4})-(\d{2})-(\d{2})$/', $text, $m) || !checkdate((int) $m[2], (int) $m[3], (int) $m[1])) {
            throw new ApiError(400, 'invalid_date', 'Dates are YYYY-MM-DD.', ['field' => 'next_at']);
        }
        return $text;
    }

    private function country(mixed $value): string
    {
        $code = strtoupper(trim((string) $value));
        if ($code !== '' && !preg_match('/^[A-Z]{2}$/', $code)) throw new ApiError(400, 'invalid_country', 'Countries are ISO codes like AT, DE, NL.', ['field' => 'country']);
        return $code;
    }

    /** @return list<string> */
    private function strings(mixed $value, bool $countries): array
    {
        $list = is_array($value) ? $value : (is_string($value) && trim($value) !== '' ? preg_split('/\s*[,;\n]\s*/', $value) : []);
        $out = [];
        foreach ($list ?: [] as $item) {
            if (!is_scalar($item)) continue;
            $text = mb_substr(trim((string) $item), 0, 1024);
            if ($text === '') continue;
            $out[] = $countries ? $this->country($text) : $text;
        }
        return array_values(array_unique(array_slice($out, 0, 100)));
    }

    /** @return list<array<string, mixed>> */
    private function listOf(mixed $value): array
    {
        if (!is_array($value)) return [];
        return array_values(array_filter($value, 'is_array'));
    }

    /** @param list<string> $allowed */
    private function invalid(string $field, array $allowed): ApiError
    {
        return new ApiError(400, "invalid_$field", "Unknown $field.", ['field' => $field, 'allowed' => $allowed]);
    }

    /** @return list<string> */
    private static function decode(mixed $json): array
    {
        $list = json_decode((string) $json, true);
        return is_array($list) ? array_values(array_map('strval', array_filter($list, 'is_scalar'))) : [];
    }

    private static function url(string $value): string
    {
        if ($value === '' || preg_match('#^[a-z][a-z0-9+.-]*:#i', $value)) return $value;
        return str_contains($value, '.') && !str_contains($value, ' ') && !str_contains($value, '@') ? "https://$value" : $value;
    }

    /** "Institut für die Zukunft " → "institutfurdiezukunft", "@trezor_sa" → "trezorsa", "The Partae" → "partae" */
    public static function nameKey(string $name): string
    {
        $text = mb_strtolower(trim($name));
        $ascii = function_exists('iconv') ? @iconv('UTF-8', 'ASCII//TRANSLIT//IGNORE', $text) : false;
        $text = is_string($ascii) && $ascii !== '' ? strtolower($ascii) : $text;
        $text = (string) preg_replace('/^(the|le|la|der|die|das)\s+/', '', $text);
        return (string) preg_replace('/[^a-z0-9]+/', '', $text);
    }
}
