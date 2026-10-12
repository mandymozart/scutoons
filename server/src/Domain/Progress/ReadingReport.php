<?php
declare(strict_types=1);

namespace Scutoons\Domain\Progress;

use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * Where readers are (BACKLOG 10.69, owner 2026-10-11: "wie weit gelesen wurde in einem Chapter – das ist ja auch sehr
 * aussagekräftig, wo eine Person ist"): read-only views on the members' save games (`progress` row `scutoons`, DATA.md
 * "The save game"). Each chapter checkpoint (`<toon>/chapter/<n>`) holds `position` (0..1, slices read ÷ the chapter's
 * slices), `isComplete` (read to its end once) and – since 10.69 – the furthest `group` / `slice` read. Guests' save
 * games live only in their browsers: they are not in here.
 */
final class ReadingReport
{
    /** steps of the "where the unfinished readers stopped" histogram (10 % each) */
    public const STEPS = 10;

    public function __construct(private readonly Database $db)
    {
    }

    /**
     * One member: the chapters they opened (last read first) and the size of the rest of their save game; null = no
     * save game on the server.
     * @return array{updated_at: ?string, chapters: list<array<string, mixed>>, variables: int, places: int, history: int}|null
     */
    public function forUser(string $userId): ?array
    {
        $row = $this->db->one('SELECT state, updated_at FROM progress WHERE user_id = ? AND toon_id = ?', [$userId, ProgressService::WORLD]);
        if (!$row) return null;
        $state = json_decode((string) $row['state']);
        $chapters = self::chapters($state);
        usort($chapters, fn(array $a, array $b) => strcmp((string) $b['read_at'], (string) $a['read_at']));
        $count = fn(string $key, ?string $sub = null): int => self::size($state instanceof stdClass ? ($sub === null ? ($state->{$key} ?? null) : ($state->{$key}->{$sub} ?? null)) : null);
        return [
            'updated_at' => $row['updated_at'] !== null ? gmdate('c', intdiv((int) $row['updated_at'], 1000)) : null,
            'chapters' => $chapters,
            'variables' => $count('variables'),
            'places' => $count('map', 'places'),
            'history' => $count('history'),
        ];
    }

    /**
     * Every chapter any member opened: how many started it, finished it, and where those still in their first
     * read-through stand (`stops[i]` = readers at i·10 % … (i+1)·10 %); `toon` narrows it to one toon.
     * @return array{readers: int, chapters: list<array<string, mixed>>}
     */
    public function chapterStats(?string $toon = null): array
    {
        $rows = $this->db->all('SELECT state FROM progress WHERE toon_id = ?', [ProgressService::WORLD]);
        $stats = [];
        foreach ($rows as $row) {
            foreach (self::chapters(json_decode((string) $row['state'])) as $c) {
                if ($toon !== null && $c['toon'] !== $toon) continue;
                $key = $c['toon'] . '/' . $c['chapter'];
                $s = $stats[$key] ?? ['toon' => $c['toon'], 'chapter' => $c['chapter'], 'started' => 0, 'finished' => 0, 'reading' => 0,
                    'stops' => array_fill(0, self::STEPS, 0), 'last_read_at' => null];
                $s['started']++;
                if ($c['finished']) {
                    $s['finished']++;
                } else {
                    $s['reading']++;
                    $s['stops'][min(self::STEPS - 1, (int) floor($c['position'] * self::STEPS))]++;
                }
                if ($c['read_at'] !== null && strcmp((string) $c['read_at'], (string) $s['last_read_at']) > 0) $s['last_read_at'] = $c['read_at'];
                $stats[$key] = $s;
            }
        }
        $list = array_values($stats);
        usort($list, fn(array $a, array $b) => [$a['toon'], $a['chapter']] <=> [$b['toon'], $b['chapter']]);
        return ['readers' => count($rows), 'chapters' => $list];
    }

    /**
     * The chapter checkpoints of a save game: [{ toon, chapter, position, finished, group, slice, started_at, read_at }]
     * (`position` as @scutoons/content checkpointPosition: without one, finished = 1, else 0; group / slice null when the
     * checkpoint is older than 10.69)
     * @return list<array<string, mixed>>
     */
    public static function chapters(mixed $state): array
    {
        $progress = $state instanceof stdClass ? ($state->progress ?? null) : null;
        if (!is_array($progress)) return [];
        $out = [];
        foreach ($progress as $c) {
            if (!$c instanceof stdClass || !is_string($c->uri ?? null)) continue;
            if (!preg_match('~^(' . ProgressService::TOON_PATTERN . ')/chapter/(\d{1,9})$~', $c->uri, $m)) continue;
            $finished = !empty($c->isComplete);
            $p = $c->position ?? null;
            $position = is_int($p) || is_float($p) ? max(0.0, min(1.0, (float) $p)) : ($finished ? 1.0 : 0.0);
            $out[] = [
                'toon' => $m[1],
                'chapter' => (int) $m[2],
                'position' => round($position, 3),
                'finished' => $finished,
                'group' => is_int($c->group ?? null) ? $c->group : null,
                'slice' => is_int($c->slice ?? null) ? $c->slice : null,
                'started_at' => is_string($c->created_at ?? null) ? $c->created_at : null,
                'read_at' => is_string($c->updated_at ?? null) ? $c->updated_at : null,
            ];
        }
        return $out;
    }

    private static function size(mixed $value): int
    {
        if (is_array($value)) return count($value);
        if ($value instanceof stdClass) return count(get_object_vars($value));
        return 0;
    }
}
