<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\Toons;
use Scutoons\Domain\Mail\MailFailed;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use stdClass;

/**
 * Review before publish (agents/CUTOVER.md 10.28 "Review levels", docs/API.md "Reviews"). Not a role – a relation per
 * toon, set by an admin (table content_review_rules, `*` = every toon):
 *   reviewer      gets the "review requested" mail, publishes a submitted version or sends it back
 *   needs_review  may save drafts and submit them, but not publish / unpublish entities of that toon
 * Only toon-level entities are reviewed (every kind with a toon in its key: toon, toon-settings, season, chapter, scene,
 * cast, credits, script …); platform-wide ones (characters, map, events …) publish directly for every creator. Without a
 * rule an account publishes directly, as before. The API decides, so the creator app and agents (the MCP server, same
 * token) get the same answer.
 *
 * A submit (table content_reviews) is pinned to one version: the reviewer publishes exactly that one, newer drafts stay
 * drafts. A newer submit for the same key annuls the open one. Submits, publishes and returns take several files at once
 * (a chapter = every language + its scenes) and send one mail per person: review_requested (reviewers),
 * review_published / review_returned (the author).
 */
final class Reviews
{
    public const REVIEWER = 'reviewer';
    public const NEEDS_REVIEW = 'needs_review';
    public const RULES = [self::REVIEWER, self::NEEDS_REVIEW];
    public const ALL_TOONS = '*';
    public const NOTE_MAX = 1000;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly EntityStore $store,
        private readonly Mailer $mailer,
        private readonly Settings $settings,
        private readonly Toons $toons,
    ) {
    }

    /** Is this entity reviewed at all? (toon-level kinds only) */
    public static function applies(EntityKey $key): bool
    {
        return $key->toon !== null;
    }

    // ── Rules (admin) ─────────────────────────────────────────────────────────────────────────────────────

    public function hasRule(string $userId, string $toon, string $rule): bool
    {
        return $this->db->value('SELECT 1 FROM content_review_rules WHERE user_id = ? AND rule = ? AND toon IN (?, ?)', [$userId, $rule, $toon, self::ALL_TOONS]) !== null;
    }

    public function needsReview(string $userId, EntityKey $key): bool
    {
        return self::applies($key) && $this->hasRule($userId, (string) $key->toon, self::NEEDS_REVIEW);
    }

    /**
     * Every rule, or the rules of one account: [{ toon, rule, user: { id, email }, created_at, created_by }]
     * @return list<array<string, mixed>>
     */
    public function rules(?string $userId = null): array
    {
        $rows = $this->db->all('SELECT r.toon, r.rule, r.user_id, u.email, r.created_at, r.created_by FROM content_review_rules r JOIN users u ON u.id = r.user_id'
            . ($userId !== null ? ' WHERE r.user_id = ?' : '') . ' ORDER BY r.toon, r.rule, u.email', $userId !== null ? [$userId] : []);
        return array_map(fn(array $r) => [
            'toon' => (string) $r['toon'],
            'rule' => (string) $r['rule'],
            'user' => ['id' => (string) $r['user_id'], 'email' => (string) $r['email']],
            'created_at' => Time::iso($r['created_at']),
            'created_by' => $r['created_by'],
        ], $rows);
    }

    /** Adds (true) or removes (false) a rule; false when nothing changed. 400 invalid_toon / invalid_rule. */
    public function setRule(string $userId, string $toon, string $rule, bool $on, ?string $by = null): bool
    {
        if ($toon !== self::ALL_TOONS && (!preg_match(EntityKey::TOON_PATTERN, $toon) || $toon === 'scutoons')) {
            throw new ApiError(400, 'invalid_toon', 'Unknown toon id (or * for every toon).');
        }
        if (!in_array($rule, self::RULES, true)) throw new ApiError(400, 'invalid_rule', 'The rule is reviewer or needs_review.');
        if (!$on) return $this->db->run('DELETE FROM content_review_rules WHERE toon = ? AND user_id = ? AND rule = ?', [$toon, $userId, $rule])->rowCount() > 0;
        if ($this->db->value('SELECT 1 FROM content_review_rules WHERE toon = ? AND user_id = ? AND rule = ?', [$toon, $userId, $rule]) !== null) return false;
        $this->db->run('INSERT INTO content_review_rules (toon, user_id, rule, created_at, created_by) VALUES (?, ?, ?, ?, ?)',
            [$toon, $userId, $rule, $this->clock->utc(), $by !== null ? mb_substr($by, 0, 255) : null]);
        return true;
    }

    /**
     * The reviewers of a toon (its own and the `*` ones), without one account (the author never reviews themselves)
     * @return list<array{id: string, email: string, locale: ?string}>
     */
    public function reviewers(string $toon, ?string $except = null): array
    {
        $rows = $this->db->all("SELECT DISTINCT u.id, u.email, u.locale FROM content_review_rules r JOIN users u ON u.id = r.user_id
            WHERE r.rule = ? AND r.toon IN (?, ?) AND u.id <> ? ORDER BY u.email", [self::REVIEWER, $toon, self::ALL_TOONS, $except ?? '']);
        return array_map(fn(array $r) => ['id' => (string) $r['id'], 'email' => (string) $r['email'], 'locale' => $r['locale'] !== null ? (string) $r['locale'] : null], $rows);
    }

    // ── Publish guard (EntitiesController publish / unpublish) ───────────────────────────────────────────

    /**
     * Throws 403 review_required when the account needs a review for this toon. A reviewer of the toon may still
     * publish a version someone else submitted (also through the plain publish endpoint).
     * @param array<string, mixed> $user
     */
    public function guardPublish(array $user, EntityKey $key, ?int $version, bool $unpublish = false): void
    {
        $userId = (string) $user['id'];
        if (!$this->needsReview($userId, $key)) return;
        if (!$unpublish && $this->hasRule($userId, (string) $key->toon, self::REVIEWER)) {
            $version ??= (int) ($this->store->latest((string) $key)['id'] ?? 0);
            $submitted = $this->db->value("SELECT 1 FROM content_reviews WHERE entity_key = ? AND version_id = ? AND state = 'pending' AND (author IS NULL OR author <> ?)",
                [(string) $key, $version, $userId]);
            if ($submitted !== null) return;
        }
        throw new ApiError(403, 'review_required', "Changes to {$key->toon} need a review – submit them for review instead.", ['toon' => $key->toon]);
    }

    /**
     * After a publish through the entity endpoint: the open review of exactly that version is done (no mail – the
     * reviews endpoint mails). Other versions' open reviews stay.
     * @param array<string, mixed> $user
     */
    public function markPublished(array $user, string $key, int $version): void
    {
        $this->db->run("UPDATE content_reviews SET state = 'published', decided_by = ?, decided_at = ? WHERE entity_key = ? AND version_id = ? AND state = 'pending'",
            [(string) $user['id'], $this->clock->utc(), $key, $version]);
    }

    // ── Use cases (/api/reviews) ──────────────────────────────────────────────────────────────────────────

    /**
     * GET /api/reviews → { needs_review: [toon|*], reviewer_of: [toon|*], to_review: [review], mine: [review] }
     * to_review = open submits of the toons this account reviews (not its own); mine = its last 50 submits.
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function overview(array $user): array
    {
        $userId = (string) $user['id'];
        $toons = fn(string $rule) => array_map('strval', array_column($this->db->all('SELECT toon FROM content_review_rules WHERE user_id = ? AND rule = ? ORDER BY toon', [$userId, $rule]), 'toon'));
        $reviewerOf = $toons(self::REVIEWER);
        $toReview = [];
        if ($reviewerOf !== []) {
            $all = in_array(self::ALL_TOONS, $reviewerOf, true);
            $in = implode(',', array_fill(0, count($reviewerOf), '?'));
            $toReview = $this->select("r.state = 'pending' AND (r.author IS NULL OR r.author <> ?)" . ($all ? '' : " AND r.toon IN ($in)") . ' ORDER BY r.id',
                $all ? [$userId] : [$userId, ...$reviewerOf]);
        }
        return [
            'status' => 'success',
            'needs_review' => $toons(self::NEEDS_REVIEW),
            'reviewer_of' => $reviewerOf,
            'to_review' => $toReview,
            'mine' => $this->select('r.author = ? ORDER BY r.id DESC LIMIT 50', [$userId]),
        ];
    }

    /**
     * POST /api/reviews { items: [{ key, version? }], note? } → { status, reviews, reviewers }
     * version: default the newest. 400 invalid_items · 404 not_found · 409 already_published (that version is live) ·
     * 409 no_reviewer { toon } (an admin has to add one) · 422 review_not_needed (a platform-wide entity).
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function submit(array $user, stdClass $body): array
    {
        $userId = (string) $user['id'];
        $versions = [];
        foreach (self::items($body) as $item) {
            $key = EntityKey::parse($item->key ?? null);
            if (!self::applies($key)) throw new ApiError(422, 'review_not_needed', "$key is published directly – no review.", ['key' => (string) $key]);
            $row = isset($item->version) ? $this->store->byId((int) $item->version) : $this->store->latest((string) $key);
            if (!$row || $row['entity_key'] !== (string) $key) throw new ApiError(404, 'not_found', "No such version of $key.", ['key' => (string) $key]);
            if ($row['status'] === 'published') throw new ApiError(409, 'already_published', "This version of $key is already published.", ['key' => (string) $key]);
            $versions[(string) $key] = [$key, (int) $row['id']];
        }
        $reviewers = [];
        foreach ($versions as [$key]) {
            $reviewers[$key->toon] ??= $this->reviewers((string) $key->toon, $userId);
            if ($reviewers[$key->toon] === []) {
                throw new ApiError(409, 'no_reviewer', "Nobody reviews {$key->toon} yet – ask an admin to add a reviewer.", ['toon' => $key->toon]);
            }
        }
        $note = self::note($body->note ?? null);
        $ids = $this->db->transaction(function () use ($versions, $userId, $note) {
            $now = $this->clock->utc();
            $ids = [];
            foreach ($versions as $k => [$key, $version]) {
                $this->db->run("UPDATE content_reviews SET state = 'annulled', decided_by = ?, decided_at = ? WHERE entity_key = ? AND state = 'pending'", [$userId, $now, $k]);
                $this->db->run('INSERT INTO content_reviews (entity_key, toon, version_id, author, note, state, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)',
                    [$k, $key->toon, $version, $userId, $note, 'pending', $now]);
                $ids[] = $this->db->lastInsertId();
            }
            return $ids;
        });
        $reviews = $this->byIds($ids);

        // one mail per reviewer with the files of the toons they review
        $mails = [];
        foreach ($reviews as $review) {
            foreach ($reviewers[$review['toon']] as $reviewer) {
                $mails[$reviewer['id']] ??= ['to' => $reviewer, 'toons' => [], 'keys' => []];
                $mails[$reviewer['id']]['toons'][$review['toon']] = true;
                $mails[$reviewer['id']]['keys'][] = (string) $review['key'];
            }
        }
        foreach ($mails as $mail) {
            $this->mail(MailTemplates::REVIEW_REQUESTED, $mail['to']['email'], $mail['to']['locale'], [
                'author' => (string) ($user['email'] ?? ''),
                'toon' => $this->toonNames(array_keys($mail['toons'])),
                'files' => self::files($mail['keys']),
                'note' => $note,
                'url' => $this->creatorUrl(),
            ]);
        }
        $emails = [];
        foreach ($reviewers as $list) foreach ($list as $r) $emails[$r['email']] = true;
        return ['status' => 'success', 'reviews' => $reviews, 'reviewers' => array_keys($emails)];
    }

    /**
     * POST /api/reviews/publish { ids, note? } – publishes every pinned version, mails each author once.
     * POST /api/reviews/return  { ids, note? } – sends them back (nothing changes on the entities), mails each author.
     * Only open reviews of toons this account reviews, never its own: 404 not_found · 409 review_closed · 403 not_reviewer.
     * @param array<string, mixed> $user
     * @return array{status: string, reviews: list<array<string, mixed>>, keys: list<string>} keys = entities that changed
     */
    public function decide(array $user, stdClass $body, bool $publish): array
    {
        $userId = (string) $user['id'];
        $ids = $body->ids ?? null;
        if (!is_array($ids) || $ids === [] || count($ids) > 200) throw new ApiError(400, 'invalid_ids', 'ids: a list of 1–200 review ids.');
        $ids = array_values(array_unique(array_map('intval', $ids)));
        $note = self::note($body->note ?? null);
        $keys = $this->db->transaction(function () use ($ids, $userId, $note, $publish) {
            $now = $this->clock->utc();
            $keys = [];
            foreach ($ids as $id) {
                $row = $this->db->one('SELECT * FROM content_reviews WHERE id = ?' . $this->db->forUpdate(), [$id]);
                if (!$row) throw new ApiError(404, 'not_found', "Unknown review $id.");
                if ($row['state'] !== 'pending') throw new ApiError(409, 'review_closed', "Review $id is {$row['state']}.", ['id' => $id, 'state' => $row['state']]);
                if ($row['author'] === $userId || !$this->hasRule($userId, (string) $row['toon'], self::REVIEWER)) {
                    throw new ApiError(403, 'not_reviewer', "You don't review {$row['toon']}" . ($row['author'] === $userId ? ' – not your own submit.' : '.'), ['toon' => $row['toon']]);
                }
                if ($publish) {
                    $version = $this->store->byId((int) $row['version_id']) ?? throw new ApiError(404, 'not_found', 'The submitted version is gone.');
                    $this->store->setPublished($version);
                }
                $this->db->run('UPDATE content_reviews SET state = ?, decided_by = ?, decided_at = ?, decision_note = ? WHERE id = ?',
                    [$publish ? 'published' : 'returned', $userId, $now, $note, $id]);
                $keys[] = (string) $row['entity_key'];
            }
            return $keys;
        });
        $reviews = $this->byIds($ids);

        $byAuthor = [];
        foreach ($reviews as $review) {
            if ($review['author'] === null) continue;
            $byAuthor[$review['author']['id']][] = $review;
        }
        foreach ($byAuthor as $authorId => $list) {
            $author = $this->db->one('SELECT email, locale FROM users WHERE id = ?', [$authorId]);
            if (!$author) continue;
            $toons = array_values(array_unique(array_column($list, 'toon')));
            $this->mail($publish ? MailTemplates::REVIEW_PUBLISHED : MailTemplates::REVIEW_RETURNED, (string) $author['email'], $author['locale'] ?? null, [
                'reviewer' => (string) ($user['email'] ?? ''),
                'toon' => $this->toonNames($toons),
                'files' => self::files(array_column($list, 'key')),
                'note' => $note,
                'url' => $this->creatorUrl(),
            ]);
        }
        return ['status' => 'success', 'reviews' => $reviews, 'keys' => array_values(array_unique($keys))];
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /**
     * @param list<int> $ids
     * @return list<array<string, mixed>>
     */
    private function byIds(array $ids): array
    {
        if ($ids === []) return [];
        return $this->select('r.id IN (' . implode(',', array_fill(0, count($ids), '?')) . ') ORDER BY r.id', $ids);
    }

    /**
     * Reviews as JSON: { id, key, toon, locale, version, newest (the entity's newest version – newer = not part of the
     * review), state, note, author: {id, email}|null, created_at, decided_by: {id, email}|null, decided_at, decision_note }
     * @param list<mixed> $params
     * @return list<array<string, mixed>>
     */
    private function select(string $where, array $params): array
    {
        $rows = $this->db->all("SELECT r.*, a.email AS author_email, d.email AS decided_email,
                (SELECT MAX(v.id) FROM entity_versions v WHERE v.entity_key = r.entity_key) AS newest
            FROM content_reviews r LEFT JOIN users a ON a.id = r.author LEFT JOIN users d ON d.id = r.decided_by WHERE $where", $params);
        return array_map(fn(array $r) => [
            'id' => (int) $r['id'],
            'key' => (string) $r['entity_key'],
            'toon' => (string) $r['toon'],
            'locale' => EntityKey::parse((string) $r['entity_key'])->locale,
            'version' => (int) $r['version_id'],
            'newest' => $r['newest'] !== null ? (int) $r['newest'] : null,
            'state' => (string) $r['state'],
            'note' => $r['note'],
            'author' => $r['author'] !== null ? ['id' => (string) $r['author'], 'email' => $r['author_email']] : null,
            'created_at' => Time::iso($r['created_at']),
            'decided_by' => $r['decided_by'] !== null ? ['id' => (string) $r['decided_by'], 'email' => $r['decided_email']] : null,
            'decided_at' => Time::iso($r['decided_at']),
            'decision_note' => $r['decision_note'],
        ], $rows);
    }

    /** @return list<stdClass> */
    private static function items(stdClass $body): array
    {
        $items = $body->items ?? null;
        if (!is_array($items) || $items === [] || count($items) > 200) throw new ApiError(400, 'invalid_items', 'items: a list of 1–200 { key, version? }.');
        foreach ($items as $item) {
            if (!$item instanceof stdClass) throw new ApiError(400, 'invalid_items', 'items: a list of 1–200 { key, version? }.');
        }
        return array_values($items);
    }

    private static function note(mixed $value): ?string
    {
        if (!is_string($value)) return null;
        $value = trim($value);
        return $value === '' ? null : mb_substr($value, 0, self::NOTE_MAX);
    }

    /**
     * Readable file list for the mails: one entry per entity with its languages – "chapter 2 (de, en), scene 2/kiosk (de)"
     * @param list<string> $keys
     */
    public static function files(array $keys): string
    {
        $entities = [];
        foreach ($keys as $k) {
            $key = EntityKey::parse($k);
            $label = str_replace('-', ' ', $key->kind) . ($key->number !== null ? ' ' . $key->number : '') . ($key->scene !== null ? '/' . $key->scene : '');
            $entities[$label] ??= [];
            if ($key->locale !== null) $entities[$label][] = $key->locale;
        }
        $parts = [];
        foreach ($entities as $label => $locales) $parts[] = $label . ($locales !== [] ? ' (' . implode(', ', array_unique($locales)) . ')' : '');
        return implode(', ', $parts);
    }

    /** The creator's dashboard: "To review" for reviewers, "Your submissions" for authors (owner 2026-10-04) */
    private function creatorUrl(): string
    {
        return rtrim($this->settings->get('CREATOR_URL'), '/') . '/';
    }

    /**
     * The toons' names for the mails ("Cat Pope"; owner 2026-10-04), the id when the catalog has none
     * @param list<string|int> $ids
     */
    private function toonNames(array $ids): string
    {
        return implode(', ', array_map(function (string|int $id): string {
            $name = $this->toons->find((string) $id)['name'] ?? null;
            return is_string($name) && $name !== '' ? $name : (string) $id;
        }, $ids));
    }

    /** @param array<string, mixed> $vars */
    private function mail(string $template, string $to, mixed $locale, array $vars): void
    {
        try {
            $this->mailer->send($template, $to, MailTemplates::locale($locale), $vars);
        } catch (MailFailed) {
            // logged (admin → Emails); the review stands
        }
    }
}
