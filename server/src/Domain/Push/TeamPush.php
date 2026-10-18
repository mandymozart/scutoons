<?php
declare(strict_types=1);

namespace Scutoons\Domain\Push;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Toons;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/**
 * Push messages for the team (agents/research/PWA-APPS.md §5, owner 2026-10-18) – each to its own tool's browsers:
 * - `embassy.due`      the Embassy reminder (EmbassyReminders, the person's own rhythm) – the push channel; `embassy.mail`
 *                      is the same reminder's mail channel (a preference only, no push)
 * - `admin.proposals`  a new admin proposal → every admin except its author (Proposals::create)
 * - `creator.reviews`  a publishing request → the toon's reviewers (Reviews::submit, the same people the mail goes to)
 * - `creator.drafts`   drafts someone else saved and nobody published / submitted → only the toon's reviewers, never the
 *                      person who saved them (`cron()`, GET|POST /api/push/cron?key=PUSH_CRON_KEY)
 * Rules: sent right away (no quiet hours); never to the person who made the change; once per thing (`push_notified`);
 * one notification per kind on a phone (its `tag` – a newer one replaces it); the person's own switch per kind
 * (`push_prefs`, no row = on). Sending never breaks what triggered it: no VAPID keys / no browsers = nothing happens.
 */
final class TeamPush
{
    /** kind → the app it belongs to and whether it is a push (embassy.mail is the reminder's mail channel) */
    public const KINDS = [
        'embassy.due' => ['app' => 'embassy', 'push' => true],
        'embassy.mail' => ['app' => 'embassy', 'push' => false],
        'admin.proposals' => ['app' => 'admin', 'push' => true],
        'creator.reviews' => ['app' => 'creator', 'push' => true],
        'creator.drafts' => ['app' => 'creator', 'push' => true],
    ];
    /** a draft is told about once its newest version rests this long (nobody is in the middle of saving it) */
    public const DRAFT_SETTLE_MINUTES = 30;
    /** drafts older than this are not told about any more (the first run doesn't dig up old ones) */
    public const DRAFT_LOOKBACK_DAYS = 7;

    private const TEXTS = [
        'de' => [
            'embassy.due' => ['title' => 'Embassy', 'today' => 'Heute sind {n} Kontakte dran.', 'week' => 'Diese Woche sind {n} Kontakte dran.', 'one_today' => 'Heute ist ein Kontakt dran.', 'one_week' => 'Diese Woche ist ein Kontakt dran.'],
            'admin.proposals' => ['title' => 'Neuer Vorschlag', 'body' => '{title} – von {who}'],
            'creator.reviews' => ['title' => 'Zur Freigabe: {toon}', 'body' => '{who} bittet dich um Freigabe ({n} Dateien).', 'one' => '{who} bittet dich um Freigabe.'],
            'creator.drafts' => ['title' => '{toon}: Entwürfe warten', 'body' => '{n} Entwürfe von {who} sind noch nicht veröffentlicht.', 'one' => 'Ein Entwurf von {who} ist noch nicht veröffentlicht.'],
            'test' => 'Test: so sieht diese Nachricht aus.',
        ],
        'en' => [
            'embassy.due' => ['title' => 'Embassy', 'today' => '{n} contacts are due today.', 'week' => '{n} contacts are due this week.', 'one_today' => 'One contact is due today.', 'one_week' => 'One contact is due this week.'],
            'admin.proposals' => ['title' => 'New proposal', 'body' => '{title} – by {who}'],
            'creator.reviews' => ['title' => 'To review: {toon}', 'body' => '{who} asks you to publish {n} files.', 'one' => '{who} asks you to publish.'],
            'creator.drafts' => ['title' => '{toon}: drafts waiting', 'body' => '{n} drafts by {who} are not published yet.', 'one' => 'A draft by {who} is not published yet.'],
            'test' => 'Test: this is how this message looks.',
        ],
    ];

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly PushSender $sender,
        private readonly PushSubscriptions $subscriptions,
        private readonly Roles $roles,
        private readonly Toons $toons,
    ) {
    }

    // ── preferences ────────────────────────────────────────────────────────────────────────────────────────

    /** the person's switch for a kind (no row = on) */
    public function on(string $userId, string $kind): bool
    {
        $value = $this->db->value('SELECT `on` FROM push_prefs WHERE user_id = ? AND kind = ?', [$userId, $kind]);
        return $value === null || (int) $value === 1;
    }

    /**
     * The kinds a person may get in an app (by role) – the tools' "Benachrichtigungen" page.
     * @param array<string, mixed> $user
     * @return list<string>
     */
    public function kindsFor(array $user, string $app): array
    {
        $roles = $this->roles->of((string) $user['id']);
        $has = static fn(string ...$any): bool => array_intersect($any, $roles) !== [];
        // who uses which tool (shared/auth roles.ts APP_ROLES)
        $allowed = ['embassy' => $has('liaison', 'ambassador', Roles::ADMIN), 'admin' => $has(Roles::ADMIN), 'creator' => $has(...Roles::CONTENT_EDITORS)][$app] ?? false;
        if (!$allowed) return [];
        return array_keys(array_filter(self::KINDS, static fn(array $meta): bool => $meta['app'] === $app));
    }

    /**
     * GET /api/push/prefs?app=… → the push key, this account's browsers in the app and its kinds with their switches
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function prefs(array $user, string $app): array
    {
        self::app($app);
        $userId = (string) $user['id'];
        return [
            'key' => $this->sender->configured() ? $this->sender->publicKey() : null,
            'kinds' => array_map(fn(string $kind): array => ['kind' => $kind, 'on' => $this->on($userId, $kind), 'push' => self::KINDS[$kind]['push']], $this->kindsFor($user, $app)),
        ];
    }

    /**
     * PUT /api/push/prefs { kind, on }
     * @param array<string, mixed> $user
     */
    public function setPref(array $user, mixed $kind, mixed $on): void
    {
        if (!is_string($kind) || !isset(self::KINDS[$kind])) throw new ApiError(400, 'invalid_kind', 'Unknown notification kind.');
        if (!is_bool($on)) throw new ApiError(400, 'invalid_value', '`on` must be true or false.');
        if (!in_array($kind, $this->kindsFor($user, self::KINDS[$kind]['app']), true)) throw new ApiError(403, 'forbidden', 'Not for this account.');
        $this->db->run('INSERT INTO push_prefs (user_id, kind, `on`, updated_at) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE `on` = VALUES(`on`), updated_at = VALUES(updated_at)',
            [(string) $user['id'], $kind, $on ? 1 : 0, $this->clock->utc()]);
    }

    // ── the kinds ─────────────────────────────────────────────────────────────────────────────────────────

    /**
     * The Embassy reminder's push (EmbassyReminders, when the person's rhythm says so) – once per local day
     * @return bool something reached the person's browsers
     */
    public function embassyDue(string $userId, int $count, bool $daily, string $today): bool
    {
        if ($count <= 0) return false;
        $result = $this->notify('embassy.due', [$userId], "day:$today", function (string $locale) use ($count, $daily): array {
            $t = self::texts($locale)['embassy.due'];
            $body = $count === 1 ? $t[$daily ? 'one_today' : 'one_week'] : strtr($t[$daily ? 'today' : 'week'], ['{n}' => (string) $count]);
            return ['title' => $t['title'], 'body' => $body, 'url' => '/', 'tag' => 'embassy.due'];
        });
        return $result['delivered'] > 0;
    }

    /**
     * A new admin proposal → every admin except its author
     * @param array<string, mixed> $proposal
     * @param array<string, mixed> $author
     */
    public function proposalCreated(array $proposal, array $author): void
    {
        $admins = array_map(static fn(array $r): string => (string) $r['user_id'], $this->db->all(
            "SELECT ur.user_id FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE r.name = ? AND ur.user_id <> ?", [Roles::ADMIN, (string) $author['id']]
        ));
        $who = $this->name($author);
        $title = (string) ($proposal['title'] ?? '');
        $id = (string) ($proposal['id'] ?? '');
        $this->notify('admin.proposals', $admins, "proposal:$id", static function (string $locale) use ($who, $title, $id): array {
            $t = self::texts($locale)['admin.proposals'];
            return ['title' => $t['title'], 'body' => strtr($t['body'], ['{title}' => $title, '{who}' => $who]), 'url' => '/proposals/' . rawurlencode($id), 'tag' => 'admin.proposals'];
        });
    }

    /**
     * A publishing request → the reviewers it was mailed to (Reviews::submit already leaves out the author)
     * @param list<string> $reviewerIds
     * @param list<string> $toons
     * @param list<int|string> $reviewIds
     * @param array<string, mixed> $author
     */
    public function reviewRequested(array $reviewerIds, array $toons, int $files, array $reviewIds, array $author): void
    {
        $who = $this->name($author);
        $toon = $this->toonNames($toons);
        $ref = 'reviews:' . implode(',', $reviewIds);
        $ids = array_values(array_filter($reviewerIds, static fn(string $id): bool => $id !== (string) $author['id']));
        $this->notify('creator.reviews', $ids, $ref, static function (string $locale) use ($who, $toon, $files): array {
            $t = self::texts($locale)['creator.reviews'];
            $body = $files === 1 ? $t['one'] : $t['body'];
            return ['title' => strtr($t['title'], ['{toon}' => $toon]), 'body' => strtr($body, ['{who}' => $who, '{n}' => (string) $files]), 'url' => '/', 'tag' => 'creator.reviews'];
        });
    }

    /**
     * The cron run (GET|POST /api/push/cron): drafts someone else saved – the newest version of an entity is a draft by a
     * person, newer than its published one (or none), resting DRAFT_SETTLE_MINUTES, not older than DRAFT_LOOKBACK_DAYS,
     * no open publishing request. Per toon and reviewer one message about the drafts of OTHER people they weren't told
     * about yet.
     * @return array{toons: int, notified: int, delivered: int}
     */
    public function cron(): array
    {
        $settled = $this->clock->utc(-self::DRAFT_SETTLE_MINUTES * 60);
        $since = $this->clock->utc(-self::DRAFT_LOOKBACK_DAYS * 86400);
        $rows = $this->db->all(
            "SELECT v.entity_key, v.toon, v.id, v.author
             FROM entity_versions v
             JOIN (SELECT entity_key, MAX(id) AS last, MAX(CASE WHEN status = 'published' THEN id END) AS pub
                   FROM entity_versions WHERE toon IS NOT NULL GROUP BY entity_key) k ON k.entity_key = v.entity_key AND k.last = v.id
             WHERE v.status = 'draft' AND v.author IS NOT NULL AND v.created_at <= ? AND v.created_at >= ?
               AND NOT EXISTS (SELECT 1 FROM content_reviews r WHERE r.entity_key = v.entity_key AND r.state = 'pending')
             ORDER BY v.toon, v.id",
            [$settled, $since]
        );
        $byToon = [];
        foreach ($rows as $row) $byToon[(string) $row['toon']][] = ['ref' => $row['entity_key'] . '@' . $row['id'], 'author' => (string) $row['author']];
        $result = ['toons' => count($byToon), 'notified' => 0, 'delivered' => 0];
        foreach ($byToon as $toon => $drafts) {
            foreach ($this->reviewersOf((string) $toon) as $reviewer) {
                // never about their own drafts; only what they weren't told about yet
                $theirs = array_values(array_filter($drafts, fn(array $d): bool => $d['author'] !== $reviewer && !$this->told($reviewer, 'creator.drafts', $d['ref'])));
                if ($theirs === []) continue;
                $authors = array_values(array_unique(array_map(static fn(array $d): string => $d['author'], $theirs)));
                $who = implode(', ', array_map(fn(string $id): string => $this->name($this->userRow($id)), $authors));
                $name = $this->toonNames([(string) $toon]);
                $count = count($theirs);
                $sent = $this->notify('creator.drafts', [$reviewer], 'toon:' . $toon . ':' . $theirs[count($theirs) - 1]['ref'], static function (string $locale) use ($name, $who, $count, $toon): array {
                    $t = self::texts($locale)['creator.drafts'];
                    $body = $count === 1 ? $t['one'] : $t['body'];
                    return ['title' => strtr($t['title'], ['{toon}' => $name]), 'body' => strtr($body, ['{who}' => $who, '{n}' => (string) $count]), 'url' => '/' . rawurlencode((string) $toon), 'tag' => 'creator.drafts:' . $toon];
                });
                if ($sent['recipients'] > 0 || !$this->on($reviewer, 'creator.drafts')) {
                    // told (or chose not to be): these drafts are done for this person
                    foreach ($theirs as $d) $this->remember($reviewer, 'creator.drafts', $d['ref']);
                }
                $result['notified'] += $sent['recipients'] > 0 ? 1 : 0;
                $result['delivered'] += $sent['delivered'];
            }
        }
        return $result;
    }

    /**
     * The admin's test (admin → Emails → Push): a sample of the kind to the admin's own browsers in its app – no
     * preference, no "told once"
     * @param array<string, mixed> $admin
     * @return array{recipients: int, delivered: int, gone: int, failed: int}
     */
    public function test(array $admin, mixed $kind): array
    {
        if (!is_string($kind) || !(self::KINDS[$kind]['push'] ?? false)) throw new ApiError(400, 'invalid_kind', 'Unknown notification kind.');
        $app = self::KINDS[$kind]['app'];
        $recipients = $this->subscriptions->recipients((string) $admin['id'], $app);
        if ($recipients === []) throw new ApiError(409, 'no_browser', "No browser of yours gets $app pushes yet – switch them on in the $app app (Benachrichtigungen).");
        $result = $this->sender->send($recipients, static function (string $locale) use ($kind): array {
            $t = self::texts($locale);
            $title = is_string($t[$kind]['title'] ?? null) ? strtr((string) $t[$kind]['title'], ['{toon}' => 'SCUTOONS']) : 'SCUTOONS';
            return ['title' => $title, 'body' => $t['test'], 'url' => '/', 'tag' => $kind];
        });
        $this->log($kind, $result, true, (string) $admin['id']);
        return $result;
    }

    // ── sending ───────────────────────────────────────────────────────────────────────────────────────────

    /**
     * One kind to some people: only those whose switch is on, who weren't told about `ref` yet and have a browser in the
     * kind's app. Never throws for push trouble (no keys → nothing).
     * @param list<string> $userIds
     * @param callable(string): array<string, mixed> $payload
     * @return array{recipients: int, delivered: int, gone: int, failed: int}
     */
    public function notify(string $kind, array $userIds, string $ref, callable $payload): array
    {
        $total = ['recipients' => 0, 'delivered' => 0, 'gone' => 0, 'failed' => 0];
        if (!$this->sender->configured() || !(self::KINDS[$kind]['push'] ?? false)) return $total;
        $app = self::KINDS[$kind]['app'];
        foreach (array_unique($userIds) as $userId) {
            if (!$this->on($userId, $kind) || $this->told($userId, $kind, $ref)) continue;
            $recipients = $this->subscriptions->recipients($userId, $app);
            if ($recipients === []) continue;
            try {
                $result = $this->sender->send($recipients, $payload);
            } catch (ApiError) {
                continue;
            }
            $this->remember($userId, $kind, $ref);
            foreach ($result as $k => $n) $total[$k] += $n;
        }
        if ($total['recipients'] > 0) $this->log($kind, $total, false, null);
        return $total;
    }

    private function told(string $userId, string $kind, string $ref): bool
    {
        return $this->db->value('SELECT 1 FROM push_notified WHERE user_id = ? AND kind = ? AND ref = ?', [$userId, $kind, mb_substr($ref, 0, 191)]) !== null;
    }

    private function remember(string $userId, string $kind, string $ref): void
    {
        $this->db->run('INSERT IGNORE INTO push_notified (user_id, kind, ref, sent_at) VALUES (?, ?, ?, ?)', [$userId, $kind, mb_substr($ref, 0, 191), $this->clock->utc()]);
    }

    /** @param array{recipients: int, delivered: int, gone: int, failed: int} $result */
    private function log(string $kind, array $result, bool $test, ?string $by): void
    {
        $this->db->run('INSERT INTO push_sends (kind, toon, chapter, test, recipients, delivered, gone, failed, sent_by, sent_at) VALUES (?, NULL, NULL, ?, ?, ?, ?, ?, ?, ?)',
            [$kind, $test ? 1 : 0, $result['recipients'], $result['delivered'], $result['gone'], $result['failed'], $by, $this->clock->utc()]);
    }

    // ── helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** @return list<string> the reviewers of a toon (its own and the `*` ones) */
    private function reviewersOf(string $toon): array
    {
        return array_map(static fn(array $r): string => (string) $r['user_id'], $this->db->all(
            "SELECT DISTINCT user_id FROM content_review_rules WHERE rule = 'reviewer' AND toon IN (?, '*')", [$toon]
        ));
    }

    /** @return array<string, mixed> */
    private function userRow(string $id): array
    {
        return $this->db->one('SELECT id, email, display_name FROM users WHERE id = ?', [$id]) ?? ['id' => $id];
    }

    /**
     * a person's name for a message: their reader name, else the part of the mail address before the @
     * @param array<string, mixed> $user
     */
    private function name(array $user): string
    {
        $name = trim((string) ($user['display_name'] ?? ''));
        if ($name !== '') return $name;
        $email = (string) ($user['email'] ?? '');
        return $email !== '' ? explode('@', $email)[0] : '?';
    }

    /** @param list<string> $ids */
    private function toonNames(array $ids): string
    {
        return implode(', ', array_map(function (string $id): string {
            $name = $id === '*' ? null : ($this->toons->find($id)['name'] ?? null);
            return is_string($name) && $name !== '' ? $name : $id;
        }, array_values(array_unique($ids))));
    }

    /** @return array<string, mixed> */
    private static function texts(string $locale): array
    {
        return self::TEXTS[$locale] ?? self::TEXTS['de'];
    }

    public static function app(mixed $app): string
    {
        if (!is_string($app) || !in_array($app, ['creator', 'admin', 'embassy'], true)) throw new ApiError(400, 'invalid_app', 'Unknown app.');
        return $app;
    }
}
