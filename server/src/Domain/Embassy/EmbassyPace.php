<?php
declare(strict_types=1);

namespace Scutoons\Domain\Embassy;

use DateTimeImmutable;
use DateTimeZone;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use Throwable;

/**
 * Pace + the weekly portion (owner 2026-10-05; docs/API.md "Embassy" → "Pace"): the Embassy work is done by part-time
 * liaisons at their own pace. Each person sets how many contacts a week they take on (`per_week`, preset PRESET, MIN–MAX)
 * and a reminder mail to themselves (`weekly` = Monday, `daily` = Mon–Fri, `off`) – the onboarding is the first
 * PUT /embassy/pace. Every CHECKIN_DAYS days the app asks whether the pace is right (less / fine / more = −5 / 0 / +5).
 *
 * The weekly portion (GET /embassy/portion): the week is Monday–Sunday in the API's time zone; `done` = organisations
 * the person contacted this week (CONTACT_KINDS by them – mails sent from the app count), `left` = pace − done, and
 * up to `left` work items, one per organisation, in this order: `due` (their leads with a next date today or earlier,
 * oldest first), `quiet` ("answered before, quiet now", Embassy::quietCondition), `first` (research / ready leads never
 * contacted, tier A first). Do-not-contact organisations never show. `today_count` = today's share of the rest of the
 * week (working days Mon–Fri). Each item carries a structured `hint` (HINTS) the app renders in de / en.
 *
 * Tables embassy_pace + embassy_checkins (migration 2026-10-05-embassy-pace); without them every endpoint here answers
 * 503 not_available and /embassy/me says `pace: null`. Nothing here ever mails a contact – reminders (EmbassyReminders)
 * go only to the team member.
 */
final class EmbassyPace
{
    public const PRESET = 10;
    public const MIN = 1;
    public const MAX = 100;
    /** Check-in: less / fine / more → per week −5 / 0 / +5 (MIN … MAX) */
    public const ANSWERS = ['less' => -5, 'fine' => 0, 'more' => 5];
    public const CHECKIN_DAYS = 28;
    public const REMINDERS = ['weekly', 'daily', 'off'];
    /** Activities that count as a contact made (`done`); a note or a reply does not */
    public const CONTACT_KINDS = ['mail_out', 'call', 'dm', 'meeting'];
    public const GROUPS = ['due', 'quiet', 'first'];
    /**
     * Hint kinds (docs/API.md "Pace"): due { date } · mail_no_answer { days, subject? } · voicemail { days } · no_answer
     * { days } · call_back { date } · replied { days, subject? } · answered_then_quiet { date, days } · no_contact_yet
     */
    public const HINTS = ['due', 'mail_no_answer', 'voicemail', 'no_answer', 'call_back', 'replied', 'answered_then_quiet', 'no_contact_yet'];
    /** Candidates read per group (one person never owns more leads than that in one group) */
    private const CANDIDATES = 500;

    /** The tables exist (migration 2026-10-05-embassy-pace) – null = not checked yet */
    private ?bool $available = null;

    public function __construct(private readonly Embassy $embassy, private readonly Database $db, private readonly Clock $clock)
    {
    }

    public function available(): bool
    {
        if ($this->available === null) {
            try {
                $this->db->value('SELECT COUNT(*) FROM embassy_pace WHERE 1 = 0');
                $this->db->value('SELECT COUNT(*) FROM embassy_checkins WHERE 1 = 0');
                $this->available = true;
            } catch (Throwable) {
                $this->available = false;
            }
        }
        return $this->available;
    }

    /**
     * A person's pace – null: not onboarded yet (or no migration)
     * @return array{per_week: int, reminder: string, onboarded_at: ?string, asked_at: ?string, checkin_due: bool}|null
     */
    public function of(string $userId): ?array
    {
        if (!$this->available()) return null;
        $row = $this->db->one('SELECT * FROM embassy_pace WHERE user_id = ?', [$userId]);
        return $row !== null ? $this->pace($row) : null;
    }

    /**
     * PUT /embassy/pace: the onboarding (first call: onboarded_at = asked_at = now) or a change. Body `{ per_week?,
     * reminder? }` – what is left out stays (first call: PRESET, weekly).
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array{pace: array<string, mixed>|null}
     */
    public function set(array $data, array $user): array
    {
        $this->embassy->access($user);
        $this->requireAvailable();
        $id = (string) $user['id'];
        $row = $this->db->one('SELECT * FROM embassy_pace WHERE user_id = ?', [$id]);
        $perWeek = array_key_exists('per_week', $data) ? self::perWeek($data['per_week']) : (int) ($row['per_week'] ?? self::PRESET);
        $reminder = array_key_exists('reminder', $data) ? self::reminder($data['reminder']) : (string) ($row['reminder'] ?? 'weekly');
        $now = $this->clock->utc();
        if ($row === null) {
            $this->db->run('INSERT INTO embassy_pace (user_id, per_week, reminder, onboarded_at, asked_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)',
                [$id, $perWeek, $reminder, $now, $now, $now]);
        } else {
            $this->db->run('UPDATE embassy_pace SET per_week = ?, reminder = ?, updated_at = ? WHERE user_id = ?', [$perWeek, $reminder, $now, $id]);
        }
        return ['pace' => $this->of($id)];
    }

    /**
     * POST /embassy/pace/checkin `{ answer: less | fine | more }`: per week −5 / 0 / +5 (MIN … MAX), asked now, the
     * answer kept in embassy_checkins. 409 not_onboarded before the onboarding.
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array{pace: array<string, mixed>|null}
     */
    public function checkin(array $data, array $user): array
    {
        $this->embassy->access($user);
        $this->requireAvailable();
        $answer = is_string($data['answer'] ?? null) ? $data['answer'] : '';
        if (!isset(self::ANSWERS[$answer])) {
            throw new ApiError(400, 'invalid_answer', 'Unknown answer.', ['field' => 'answer', 'allowed' => array_keys(self::ANSWERS)]);
        }
        $id = (string) $user['id'];
        $this->db->transaction(function () use ($id, $answer): void {
            $row = $this->db->one('SELECT per_week FROM embassy_pace WHERE user_id = ?' . $this->db->forUpdate(), [$id])
                ?? throw new ApiError(409, 'not_onboarded', 'Set your pace first (PUT /embassy/pace).');
            $before = (int) $row['per_week'];
            $after = max(self::MIN, min(self::MAX, $before + self::ANSWERS[$answer]));
            $now = $this->clock->utc();
            $this->db->run('UPDATE embassy_pace SET per_week = ?, asked_at = ?, updated_at = ? WHERE user_id = ?', [$after, $now, $now, $id]);
            $this->db->run('INSERT INTO embassy_checkins (user_id, at, answer, per_week_before, per_week_after) VALUES (?, ?, ?, ?, ?)',
                [$id, $now, $answer, $before, $after]);
        });
        return ['pace' => $this->of($id)];
    }

    /**
     * GET /embassy/portion?owner=me|<user id>: somebody else's only for ambassadors (403 forbidden { role })
     * @param array<string, mixed> $user
     * @return array<string, mixed> like portionOf()
     */
    public function portion(string $owner, array $user): array
    {
        $access = $this->embassy->access($user);
        $this->requireAvailable();
        $ownerId = $owner === '' || $owner === 'me' ? (string) $user['id'] : $owner;
        if ($ownerId !== (string) $user['id']) {
            if ($access !== Embassy::AMBASSADOR) {
                throw new ApiError(403, 'forbidden', "Only ambassadors see somebody else's portion.", ['role' => Embassy::AMBASSADOR]);
            }
            if ($this->db->value('SELECT 1 FROM users WHERE id = ?', [$ownerId]) === null) throw new ApiError(404, 'not_found', 'No such user.');
        }
        return $this->portionOf($ownerId);
    }

    /**
     * One person's portion this week (no access check – the reminders use it too)
     * @return array{today: string, week: array{start: string, end: string, pace: int, done: int, left: int}, today_count: int, today_done: int,
     *               items: list<array<string, mixed>>, more: array{due: int, quiet: int, first: int}, owned: int}
     */
    public function portionOf(string $userId): array
    {
        $today = $this->embassy->todayDate();
        [$start, $end] = self::week($today);
        $pace = $this->of($userId)['per_week'] ?? self::PRESET;
        $done = $this->done($userId, $start, self::addDays($end, 1));
        $todayDone = $this->done($userId, $today, self::addDays($today, 1));
        $left = max(0, $pace - $done);

        // one item per organisation, the most urgent group first
        $seen = [];
        $items = [];
        $more = ['due' => 0, 'quiet' => 0, 'first' => 0];
        foreach (self::GROUPS as $group) {
            foreach ($this->candidates($userId, $group, $today) as $row) {
                $orgId = (string) $row['org_id'];
                if (isset($seen[$orgId])) continue;
                $seen[$orgId] = true;
                if (count($items) >= $left) {
                    $more[$group]++;
                    continue;
                }
                $item = $this->embassy->item((string) $row['id']);
                $items[] = ['group' => $group, 'hint' => $this->hint($group, $item, $today)] + $item;
            }
        }

        // today's share of what was left this morning, spread over the working days left (Mon–Fri; weekend: all of it)
        $weekday = (int) (new DateTimeImmutable($today))->format('N');
        $leftMorning = max(0, $pace - ($done - $todayDone));
        $share = $weekday >= 6 ? $leftMorning : (int) ceil($leftMorning / (6 - $weekday));
        $owned = (int) $this->db->value('SELECT COUNT(*) FROM embassy_leads WHERE owner_id = ?', [$userId]);
        return [
            'today' => $today,
            'week' => ['start' => $start, 'end' => $end, 'pace' => $pace, 'done' => $done, 'left' => $left],
            'today_count' => min(count($items), max(0, $share - $todayDone)),
            'today_done' => $todayDone,
            'items' => $items,
            'more' => $more,
            'owned' => $owned,
        ];
    }

    /**
     * GET /embassy/team/pace (ambassadors): everyone with an Embassy role – pace, leads owned, contacts of the last 4
     * weeks (oldest first, this week last), the last check-in
     * @param array<string, mixed> $user
     * @return array{items: list<array<string, mixed>>}
     */
    public function team(array $user): array
    {
        if ($this->embassy->access($user) !== Embassy::AMBASSADOR) {
            throw new ApiError(403, 'forbidden', 'Only ambassadors may do this.', ['role' => Embassy::AMBASSADOR]);
        }
        $this->requireAvailable();
        [$start] = self::week($this->embassy->todayDate());
        $items = [];
        foreach ($this->embassy->team() as $member) {
            $weeks = [];
            for ($i = 3; $i >= 0; $i--) {
                $from = self::addDays($start, -7 * $i);
                $weeks[] = ['start' => $from, 'done' => $this->done($member['id'], $from, self::addDays($from, 7))];
            }
            $checkin = $this->db->one('SELECT * FROM embassy_checkins WHERE user_id = ? ORDER BY at DESC, id DESC LIMIT 1', [$member['id']]);
            $items[] = [
                'user' => $member,
                'pace' => $this->of($member['id']),
                'owned' => (int) $this->db->value('SELECT COUNT(*) FROM embassy_leads WHERE owner_id = ?', [$member['id']]),
                'weeks' => $weeks,
                'last_checkin' => $checkin === null ? null : [
                    'at' => Time::iso($checkin['at']), 'answer' => (string) $checkin['answer'],
                    'per_week_before' => (int) $checkin['per_week_before'], 'per_week_after' => (int) $checkin['per_week_after'],
                ],
            ];
        }
        return ['items' => $items];
    }

    /**
     * Everyone whose reminder is on, with their address and account language (EmbassyReminders)
     * @return list<array{user_id: string, email: string, locale: string, reminder: string, reminded_at: ?string}>
     */
    public function reminderRows(): array
    {
        if (!$this->available()) return [];
        $rows = $this->db->all("SELECT p.user_id, p.reminder, p.reminded_at, u.email, u.locale FROM embassy_pace p JOIN users u ON u.id = p.user_id
            WHERE p.reminder <> 'off' ORDER BY u.email");
        return array_map(fn(array $r) => ['user_id' => (string) $r['user_id'], 'email' => (string) $r['email'], 'locale' => (string) ($r['locale'] ?? ''),
            'reminder' => (string) $r['reminder'], 'reminded_at' => $r['reminded_at'] !== null ? (string) $r['reminded_at'] : null], $rows);
    }

    public function markReminded(string $userId): void
    {
        $this->db->run('UPDATE embassy_pace SET reminded_at = ? WHERE user_id = ?', [$this->clock->utc(), $userId]);
    }

    /**
     * Monday and Sunday of the week of $date (YYYY-MM-DD)
     * @return array{0: string, 1: string}
     */
    public static function week(string $date): array
    {
        $day = new DateTimeImmutable($date);
        $monday = $day->modify('-' . ((int) $day->format('N') - 1) . ' days');
        return [$monday->format('Y-m-d'), $monday->modify('+6 days')->format('Y-m-d')];
    }

    /** A UTC DATETIME → the local date (the API's time zone) */
    public static function localDate(string $utc): string
    {
        return (new DateTimeImmutable($utc, new DateTimeZone('UTC')))->setTimezone(new DateTimeZone(date_default_timezone_get()))->format('Y-m-d');
    }

    // ── Internals ─────────────────────────────────────────────────────────────────────────────────────────

    /** 503 before migration 2026-10-05-embassy-pace */
    private function requireAvailable(): void
    {
        if (!$this->available()) throw new ApiError(503, 'not_available', 'Pace is not set up yet (migration 2026-10-05-embassy-pace).');
    }

    /**
     * @param array<string, mixed> $row
     * @return array{per_week: int, reminder: string, onboarded_at: ?string, asked_at: ?string, checkin_due: bool}
     */
    private function pace(array $row): array
    {
        $since = strtotime((string) ($row['asked_at'] ?? $row['onboarded_at']) . ' UTC');
        return [
            'per_week' => (int) $row['per_week'], 'reminder' => (string) $row['reminder'],
            'onboarded_at' => Time::iso($row['onboarded_at']), 'asked_at' => Time::iso($row['asked_at']),
            'checkin_due' => $since !== false && $this->clock->time() >= $since + self::CHECKIN_DAYS * 86400,
        ];
    }

    /** Organisations $userId contacted from the local date $from up to (not incl.) $to */
    private function done(string $userId, string $from, string $to): int
    {
        $kinds = implode(', ', array_fill(0, count(self::CONTACT_KINDS), '?'));
        return (int) $this->db->value("SELECT COUNT(DISTINCT org_id) FROM embassy_activities WHERE by_user_id = ? AND kind IN ($kinds) AND at >= ? AND at < ?",
            [$userId, ...self::CONTACT_KINDS, self::utcStart($from), self::utcStart($to)]);
    }

    /**
     * The person's leads of one portion group, most urgent first
     * @return list<array<string, mixed>> { id, org_id }
     */
    private function candidates(string $userId, string $group, string $today): array
    {
        $base = 'SELECT l.id, l.org_id FROM embassy_leads l JOIN embassy_orgs o ON o.id = l.org_id WHERE l.owner_id = ? AND o.do_not_contact = 0';
        $limit = ' LIMIT ' . self::CANDIDATES;
        switch ($group) {
            case 'due':
                return $this->db->all("$base AND l.next_at IS NOT NULL AND l.next_at <= ? ORDER BY l.next_at, o.name_key, l.id$limit", [$userId, $today]);
            case 'quiet':
                [$sql, $params] = $this->embassy->quietCondition(Embassy::QUIET_DAYS);
                return $this->db->all("$base AND $sql ORDER BY l.last_contact_at, o.name_key, l.id$limit", [$userId, ...$params]);
            default:
                $kinds = implode(', ', array_fill(0, count(self::CONTACT_KINDS), '?'));
                return $this->db->all("$base AND l.stage IN ('research', 'ready') AND (l.next_at IS NULL OR l.next_at <= ?)
                    AND NOT EXISTS (SELECT 1 FROM embassy_activities a WHERE a.lead_id = l.id AND a.kind IN ($kinds))
                    ORDER BY CASE WHEN o.tier IS NULL THEN 1 ELSE 0 END, o.tier, l.created_at, o.name_key, l.id$limit", [$userId, $today, ...self::CONTACT_KINDS]);
        }
    }

    /**
     * Why this item is on the list (HINTS), for the app to say in de / en
     * @param array<string, mixed> $item Embassy::item()
     * @return array<string, int|string>
     */
    private function hint(string $group, array $item, string $today): array
    {
        if ($group === 'first') return ['kind' => 'no_contact_yet'];
        $lead = $item['lead'];
        if ($group === 'quiet') {
            $at = $this->db->value("SELECT MAX(a.at) FROM embassy_activities a WHERE (a.lead_id = ? OR (a.lead_id IS NULL AND a.org_id = ?))
                AND (a.kind IN ('mail_in', 'meeting') OR (a.kind = 'call' AND a.outcome = 'reached'))", [$lead['id'], $lead['org_id']]);
            $date = is_string($at) ? self::localDate($at) : $today;
            return ['kind' => 'answered_then_quiet', 'date' => $date, 'days' => self::days($date, $today)];
        }
        $next = (string) ($lead['next_at'] ?? $today);
        $last = $item['last_activity'];
        if (is_array($last)) {
            $days = self::days(self::localDate((string) $last['at']), $today);
            $subject = isset($last['subject']) && $last['subject'] !== '' ? ['subject' => (string) $last['subject']] : [];
            switch (true) {
                case $last['kind'] === 'call' && $last['outcome'] === 'call_back':
                    return ['kind' => 'call_back', 'date' => $next];
                case $last['kind'] === 'call' && $last['outcome'] === 'voicemail':
                    return ['kind' => 'voicemail', 'days' => $days];
                case $last['kind'] === 'call' && $last['outcome'] === 'no_answer':
                    return ['kind' => 'no_answer', 'days' => $days];
                case $last['kind'] === 'mail_out':
                    return ['kind' => 'mail_no_answer', 'days' => $days] + $subject;
                case $last['kind'] === 'mail_in':
                    return ['kind' => 'replied', 'days' => $days] + $subject;
            }
        }
        return ['kind' => 'due', 'date' => $next];
    }

    /** Local midnight of $date as UTC DATETIME (activities are stored in UTC) */
    private static function utcStart(string $date): string
    {
        return (new DateTimeImmutable("$date 00:00:00"))->setTimezone(new DateTimeZone('UTC'))->format('Y-m-d H:i:s');
    }

    private static function addDays(string $date, int $days): string
    {
        return (new DateTimeImmutable($date))->modify(($days >= 0 ? '+' : '') . $days . ' days')->format('Y-m-d');
    }

    private static function days(string $from, string $to): int
    {
        return max(0, (int) round(((int) strtotime("$to 12:00:00") - (int) strtotime("$from 12:00:00")) / 86400));
    }

    private static function perWeek(mixed $value): int
    {
        $number = is_int($value) ? $value : (is_string($value) && preg_match('/^\d{1,3}$/', trim($value)) ? (int) trim($value) : null);
        if ($number === null || $number < self::MIN || $number > self::MAX) {
            throw new ApiError(400, 'invalid_pace', 'Contacts per week: a whole number from ' . self::MIN . ' to ' . self::MAX . '.',
                ['field' => 'per_week', 'min' => self::MIN, 'max' => self::MAX]);
        }
        return $number;
    }

    private static function reminder(mixed $value): string
    {
        if (!is_string($value) || !in_array($value, self::REMINDERS, true)) {
            throw new ApiError(400, 'invalid_reminder', 'Unknown reminder.', ['field' => 'reminder', 'allowed' => self::REMINDERS]);
        }
        return $value;
    }
}
