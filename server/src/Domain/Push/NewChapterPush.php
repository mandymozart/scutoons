<?php
declare(strict_types=1);

namespace Scutoons\Domain\Push;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Library\Library;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;

/**
 * "A new chapter is out" – the only push SCUTOONS sends, and only when an admin presses Send in the admin app
 * (Emails → Push; POST /api/admin/push/new-chapter). Never automatic (owner 2026-10-16, agents/research/PWA.md §2b).
 * The chapter must be published (the public library answers it); the texts come from the library per locale; the
 * notification opens the chapter. Every send is logged in push_sends (the page warns before a chapter's second send).
 */
final class NewChapterPush
{
    private const TEXTS = [
        'de' => ['title' => '%s: neues Kapitel', 'body' => '%s – jetzt lesen'],
        'en' => ['title' => '%s: new chapter', 'body' => '%s – read it now'],
    ];

    public function __construct(
        private readonly Library $library,
        private readonly PushSender $sender,
        private readonly PushSubscriptions $subscriptions,
        private readonly Database $db,
        private readonly Clock $clock,
    ) {
    }

    /**
     * GET /api/admin/push: is push configured, the subscribers, the last sends
     * @return array<string, mixed>
     */
    public function overview(): array
    {
        $sends = $this->db->all("SELECT id, toon, chapter, test, recipients, delivered, gone, failed, sent_at FROM push_sends WHERE kind = 'new_chapter' ORDER BY id DESC LIMIT 20");
        return [
            'configured' => $this->sender->configured(),
            'subscribers' => $this->subscriptions->counts(),
            'sends' => array_map(static fn(array $r): array => [
                'id' => (int) $r['id'], 'toon' => (string) $r['toon'], 'chapter' => (int) $r['chapter'], 'test' => (bool) $r['test'],
                'recipients' => (int) $r['recipients'], 'delivered' => (int) $r['delivered'], 'gone' => (int) $r['gone'], 'failed' => (int) $r['failed'],
                'sent_at' => (string) $r['sent_at'],
            ], $sends),
        ];
    }

    /**
     * @param array<string, mixed> $body { toon, chapter, test?: bool (only the admin's own browsers), dry_run?: bool }
     * @param array<string, mixed> $admin
     * @return array<string, mixed> { notification: per locale, recipients, + delivered / gone / failed when sent }
     */
    public function send(array $body, array $admin): array
    {
        $toon = is_string($body['toon'] ?? null) && preg_match('/^[A-Za-z0-9_-]{1,64}$/', $body['toon']) ? $body['toon'] : '';
        $chapter = filter_var($body['chapter'] ?? null, FILTER_VALIDATE_INT, ['options' => ['min_range' => 0, 'max_range' => 100000]]);
        if ($toon === '' || $chapter === false) throw new ApiError(400, 'invalid_chapter', 'Pick a toon and a chapter number.');
        $test = ($body['test'] ?? false) === true;
        $notifications = [];
        foreach (PushSubscriptions::LOCALES as $locale) $notifications[$locale] = $this->notification($toon, $chapter, $locale);
        $recipients = $this->subscriptions->recipients($test ? (string) $admin['id'] : null);
        $previous = (int) $this->db->value('SELECT COUNT(*) FROM push_sends WHERE toon = ? AND chapter = ? AND test = 0', [$toon, $chapter]);
        $answer = ['notification' => $notifications, 'recipients' => count($recipients), 'sent_before' => $previous, 'configured' => $this->sender->configured()];
        if (($body['dry_run'] ?? false) === true) return $answer;

        $result = $this->sender->send($recipients, static fn(string $locale): array => $notifications[$locale] ?? $notifications['de']);
        $this->db->run('INSERT INTO push_sends (kind, toon, chapter, test, recipients, delivered, gone, failed, sent_by, sent_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', [
            'new_chapter', $toon, $chapter, $test ? 1 : 0, $result['recipients'], $result['delivered'], $result['gone'], $result['failed'], $admin['id'] ?? null, $this->clock->utc(),
        ]);
        return $answer + $result;
    }

    /** @return array{title: string, body: string, url: string, tag: string} */
    private function notification(string $toon, int $chapter, string $locale): array
    {
        // 403 / 404 when the chapter isn't published (coming soon, a draft) – such a chapter is never announced
        $doc = json_decode($this->library->chapter($toon, $chapter, $locale, false)->body, true);
        $index = json_decode($this->library->toon($toon, $locale, false)->body, true);
        $names = is_array($index['entry']['title'] ?? null) ? $index['entry']['title'] : [];
        $name = (string) ($names[$locale] ?? (reset($names) ?: $toon));
        $title = is_array($doc) && is_string($doc['title'] ?? null) && trim($doc['title']) !== '' ? trim($doc['title']) : "#$chapter";
        $texts = self::TEXTS[$locale] ?? self::TEXTS['de'];
        return [
            'title' => sprintf($texts['title'], $name),
            'body' => sprintf($texts['body'], $title),
            // unprefixed = German (the base locale); English readers get their prefix
            'url' => ($locale === 'de' ? '' : "/$locale") . "/$toon/chapter/$chapter",
            // one notification per chapter on a device: a second send replaces it instead of stacking
            'tag' => "chapter:$toon:$chapter",
        ];
    }
}
