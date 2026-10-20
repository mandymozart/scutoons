<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;
use Throwable;

/**
 * The storyboard's generations (#120, agents/research/STUDIO-IN-CREATOR.md §2.2): one row per generated still or video
 * of a shot in `storyboard_generations` – the budget ledger and the review state. The file lives on a member's machine
 * (`machine` = their label: nothing is uploaded) or in the shared work storage (`machine` = 'work', `work_path` = its
 * name there, WorkStorage).
 *
 * Here: records WITHOUT a provider job – a local generation (ComfyUI on the member's GPU, a Figma render) or an upload:
 * stored finished (`succeeded`, or `failed` with its `error`). Server-side provider jobs (#121: the registry, the budget
 * gate `402 budget_exhausted`, `job` = the provider's task id, queued → running → succeeded, estimate → cost) start
 * through their own method next to record(); until then a body with `job` or a queued / running status is refused
 * (422 provider_jobs_later).
 */
final class Generations
{
    public const KINDS = ['image', 'video'];
    public const STATUSES = ['queued', 'running', 'succeeded', 'failed', 'cancelled'];
    public const MARKS = ['review', 'rejected'];
    /** `machine` of a file in the shared work storage */
    public const WORK = 'work';
    /** the most rows one list answers (a chapter had 54 so far) */
    public const LIST_LIMIT = 5000;
    private const SHOT = '#^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$#';
    private const SLUG = '#^[a-z0-9][a-z0-9._-]{0,63}$#';
    private const MACHINE = '#^[A-Za-z0-9][A-Za-z0-9 ._-]{0,63}$#';
    /** a file in the member's workspace folder: relative, no `..` */
    private const FILE = '#^(?!.*\.\.)(?!.*//)[A-Za-z0-9_][A-Za-z0-9 ._()+/-]{0,190}$#';
    private const JSON_MAX = 65536;
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;
    /** the fields a record (POST) takes */
    private const RECORD_FIELDS = ['toon', 'chapter', 'shot', 'kind', 'take', 'parents', 'inputs', 'generator', 'model', 'mode', 'status', 'prompt', 'seed',
        'ratio', 'duration', 'ms', 'estimate', 'cost', 'currency', 'file', 'file_hash', 'machine', 'work_path', 'mark', 'approved', 'dataset', 'note', 'error', 'params'];
    /** the fields an update (PUT) takes: the review and where the file is */
    private const UPDATE_FIELDS = ['mark', 'approved', 'dataset', 'note', 'machine', 'work_path'];

    public function __construct(private readonly Database $db, private readonly Clock $clock, private readonly WorkStorage $work)
    {
    }

    /** Before migration 2026-10-20-storyboard-generations the endpoints answer 503 */
    private function requireAvailable(): void
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM storyboard_generations WHERE 1 = 0');
        } catch (Throwable) {
            throw new ApiError(503, 'not_available', 'Storyboard generations are not set up yet (migration 2026-10-20-storyboard-generations).');
        }
    }

    /**
     * A chapter's generations (optionally one shot's), oldest first
     * @return list<array<string, mixed>>
     */
    public function list(mixed $toon, mixed $chapter, mixed $shot = null): array
    {
        $this->requireAvailable();
        [$toon, $chapter] = WorkStorage::check($toon, $chapter);
        $where = 'g.toon = ? AND g.chapter = ?';
        $params = [$toon, $chapter];
        if ($shot !== null && $shot !== '') {
            $where .= ' AND g.shot = ?';
            $params[] = self::shot($shot);
        }
        $rows = $this->db->all(self::select() . " WHERE $where ORDER BY g.id LIMIT " . self::LIST_LIMIT, $params);
        return array_map(self::view(...), $rows);
    }

    /** @return array<string, mixed> one generation; 404 not_found */
    public function get(int $id): array
    {
        $this->requireAvailable();
        $row = $this->db->one(self::select() . ' WHERE g.id = ?', [$id]);
        if ($row === null) throw new ApiError(404, 'not_found', 'No such generation.');
        return self::view($row);
    }

    /**
     * Records a generation without a provider job (a local one, an upload) → the stored row. 422 invalid_generation
     * { field } for a field that doesn't fit; 422 provider_jobs_later for a provider job (#121).
     * @param array<string, mixed> $user the signed-in member
     * @param array{id: string, name: string}|null $token the AI connection it came through
     * @param array<mixed> $body
     * @return array<string, mixed>
     */
    public function record(array $user, ?array $token, array $body): array
    {
        $this->requireAvailable();
        self::onlyFields($body, self::RECORD_FIELDS, ['job' => 'Provider jobs start on the server (#121) – a record has none.']);
        [$toon, $chapter] = WorkStorage::check($body['toon'] ?? null, $body['chapter'] ?? null);
        $status = $body['status'] ?? 'succeeded';
        if (in_array($status, ['queued', 'running'], true)) {
            throw new ApiError(422, 'provider_jobs_later', 'Queued / running generations are provider jobs, started on the server (#121); record a finished one.');
        }
        if (!in_array($status, ['succeeded', 'failed', 'cancelled'], true)) self::invalid('status', 'status is succeeded, failed or cancelled.');
        $kind = $body['kind'] ?? null;
        if (!in_array($kind, self::KINDS, true)) self::invalid('kind', 'kind is image or video.');
        $generator = $body['generator'] ?? null;
        if (!is_string($generator) || !preg_match(self::SLUG, $generator)) self::invalid('generator', 'generator is a name like comfy-local, luma or upload.');
        $file = self::text($body, 'file', 191, self::FILE);
        $machine = self::text($body, 'machine', 64, self::MACHINE);
        if ($status === 'succeeded' && ($file === null || $machine === null)) self::invalid($file === null ? 'file' : 'machine', 'A finished generation names its file and the machine it is on.');
        $workPath = $this->workPath($toon, $chapter, $body);
        if ($machine === self::WORK && $workPath === null) self::invalid('work_path', 'A file in the work storage names its work_path.');
        $mark = self::mark($body);
        $now = $this->clock->utc();
        $values = [
            'toon' => $toon, 'chapter' => $chapter, 'shot' => self::shot($body['shot'] ?? null), 'kind' => $kind,
            'take' => self::text($body, 'take', 64), 'parents' => self::parents($body), 'inputs' => self::json($body, 'inputs', false),
            'generator' => $generator, 'model' => self::text($body, 'model', 128), 'mode' => self::text($body, 'mode', 64), 'job' => null,
            'status' => $status, 'prompt' => self::text($body, 'prompt', 20000), 'seed' => self::integer($body, 'seed', PHP_INT_MIN, PHP_INT_MAX),
            'ratio' => self::text($body, 'ratio', 16, '#^[1-9][0-9]{0,3}:[1-9][0-9]{0,3}$#'), 'duration' => self::number($body, 'duration', 3600),
            'ms' => self::integer($body, 'ms', 0, 2147483647), 'estimate' => self::number($body, 'estimate', 999999), 'cost' => self::number($body, 'cost', 999999),
            'currency' => self::text($body, 'currency', 3, '#^[A-Z]{3}$#'), 'file' => $file,
            'file_hash' => self::text($body, 'file_hash', 64, '#^[a-f0-9]{64}$#'), 'machine' => $machine, 'work_path' => $workPath,
            'mark' => $mark, 'marked_at' => $mark !== null ? $now : null, 'approved' => (int) self::flag($body, 'approved'), 'dataset' => (int) self::flag($body, 'dataset'),
            'note' => self::text($body, 'note', 1000), 'error' => self::text($body, 'error', 1000), 'params' => self::json($body, 'params', true),
            'created_by' => (string) $user['id'], 'via_token' => $token['id'] ?? null, 'created_at' => $now, 'updated_at' => $now,
        ];
        $columns = array_keys($values);
        $this->db->run('INSERT INTO storyboard_generations (' . implode(', ', array_map(fn($c) => "`$c`", $columns)) . ') VALUES ('
            . implode(', ', array_fill(0, count($columns), '?')) . ')', array_values($values));
        return $this->get($this->db->lastInsertId());
    }

    /**
     * The review of a generation and where its file is: `mark` (review | rejected | null), `approved`, `dataset`, `note`,
     * `machine`, `work_path` (a file of the work storage of its chapter – sharing it) → the row
     * @param array<mixed> $body
     * @return array<string, mixed>
     */
    public function update(int $id, array $body): array
    {
        $row = $this->get($id);
        self::onlyFields($body, self::UPDATE_FIELDS, ['status' => 'The status is the generator\'s (#121).', 'job' => 'The job is the generator\'s (#121).']);
        $set = [];
        if (array_key_exists('mark', $body)) {
            $mark = self::mark($body);
            if ($mark !== $row['mark']) {
                $set['mark'] = $mark;
                $set['marked_at'] = $mark !== null ? $this->clock->utc() : null;
            }
        }
        foreach (['approved', 'dataset'] as $flag) {
            if (array_key_exists($flag, $body)) $set[$flag] = (int) self::flag($body, $flag);
        }
        if (array_key_exists('note', $body)) $set['note'] = self::text($body, 'note', 1000);
        if (array_key_exists('work_path', $body)) $set['work_path'] = $this->workPath((string) $row['toon'], (int) $row['chapter'], $body);
        if (array_key_exists('machine', $body)) {
            $set['machine'] = self::text($body, 'machine', 64, self::MACHINE);
            if ($set['machine'] === null && $row['status'] === 'succeeded') self::invalid('machine', 'A finished generation names the machine its file is on.');
        }
        $machine = array_key_exists('machine', $set) ? $set['machine'] : $row['machine'];
        $workPath = array_key_exists('work_path', $set) ? $set['work_path'] : $row['work_path'];
        if ($machine === self::WORK && $workPath === null) self::invalid('work_path', 'A file in the work storage names its work_path.');
        if ($set) {
            $set['updated_at'] = $this->clock->utc();
            $this->db->run('UPDATE storyboard_generations SET ' . implode(', ', array_map(fn($c) => "`$c` = ?", array_keys($set))) . ' WHERE id = ?',
                [...array_values($set), $id]);
        }
        return $this->get($id);
    }

    private static function select(): string
    {
        return 'SELECT g.*, u.email AS author, t.name AS via FROM storyboard_generations g LEFT JOIN users u ON u.id = g.created_by LEFT JOIN api_tokens t ON t.id = g.via_token';
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    private static function view(array $row): array
    {
        $decimal = fn(mixed $v) => $v === null ? null : (float) $v;
        $json = fn(mixed $v) => $v === null ? null : json_decode((string) $v, true);
        return [
            'id' => (int) $row['id'], 'toon' => $row['toon'], 'chapter' => (int) $row['chapter'], 'shot' => $row['shot'], 'kind' => $row['kind'], 'take' => $row['take'],
            'parents' => $json($row['parents']) ?? [], 'inputs' => $json($row['inputs']), 'generator' => $row['generator'], 'model' => $row['model'], 'mode' => $row['mode'],
            'job' => $row['job'], 'status' => $row['status'], 'prompt' => $row['prompt'], 'seed' => $row['seed'] === null ? null : (int) $row['seed'], 'ratio' => $row['ratio'],
            'duration' => $decimal($row['duration']), 'ms' => $row['ms'] === null ? null : (int) $row['ms'], 'estimate' => $decimal($row['estimate']),
            'cost' => $decimal($row['cost']), 'currency' => $row['currency'], 'file' => $row['file'], 'file_hash' => $row['file_hash'], 'machine' => $row['machine'],
            'work_path' => $row['work_path'],
            'work_url' => $row['work_path'] === null ? null : WorkStorage::url((string) $row['toon'], (int) $row['chapter'], (string) $row['work_path']),
            'mark' => $row['mark'], 'marked_at' => Time::iso($row['marked_at']), 'approved' => (bool) $row['approved'], 'dataset' => (bool) $row['dataset'],
            'note' => $row['note'], 'error' => $row['error'], 'params' => $json($row['params']),
            'created_by' => $row['created_by'], 'author' => $row['author'], 'via' => $row['via'],
            'created_at' => Time::iso($row['created_at']), 'updated_at' => Time::iso($row['updated_at']),
        ];
    }

    // ── Field checks ──────────────────────────────────────────────────────────────────────────────────────

    private static function invalid(string $field, string $message): never
    {
        throw new ApiError(422, 'invalid_generation', $message, ['field' => $field]);
    }

    /**
     * @param array<mixed> $body
     * @param list<string> $allowed
     * @param array<string, string> $refused field → why
     */
    private static function onlyFields(array $body, array $allowed, array $refused): void
    {
        foreach (array_keys($body) as $field) {
            if (isset($refused[$field])) {
                throw new ApiError(422, $field === 'job' ? 'provider_jobs_later' : 'invalid_generation', $refused[$field], ['field' => $field]);
            }
            if (!in_array($field, $allowed, true)) self::invalid((string) $field, "Unknown field $field.");
        }
    }

    private static function shot(mixed $shot): string
    {
        if (!is_string($shot) || !preg_match(self::SHOT, $shot)) self::invalid('shot', 'shot is the shot id (letters, digits, . _ -).');
        return $shot;
    }

    /** @param array<mixed> $body */
    private static function text(array $body, string $field, int $max, ?string $pattern = null): ?string
    {
        $value = $body[$field] ?? null;
        if ($value === null || $value === '') return null;
        if (!is_string($value) || mb_strlen($value) > $max || ($pattern !== null && !preg_match($pattern, $value))) {
            self::invalid($field, "$field is " . ($pattern !== null ? 'not in its form' : "a text of at most $max characters") . '.');
        }
        return $value;
    }

    /** @param array<mixed> $body */
    private static function integer(array $body, string $field, int $min, int $max): ?int
    {
        $value = $body[$field] ?? null;
        if ($value === null) return null;
        if (!is_int($value) || $value < $min || $value > $max) self::invalid($field, "$field is a whole number.");
        return $value;
    }

    /** @param array<mixed> $body */
    private static function number(array $body, string $field, int $max): ?float
    {
        $value = $body[$field] ?? null;
        if ($value === null) return null;
        if ((!is_int($value) && !is_float($value)) || $value < 0 || $value > $max) self::invalid($field, "$field is a number from 0 to $max.");
        return (float) $value;
    }

    /** @param array<mixed> $body */
    private static function flag(array $body, string $field): bool
    {
        $value = $body[$field] ?? false;
        if (!is_bool($value)) self::invalid($field, "$field is true or false.");
        return $value;
    }

    /** @param array<mixed> $body */
    private static function mark(array $body): ?string
    {
        $mark = $body['mark'] ?? null;
        if ($mark !== null && !in_array($mark, self::MARKS, true)) self::invalid('mark', 'mark is review, rejected or null.');
        return $mark;
    }

    /** @param array<mixed> $body the generation ids it was made from, as JSON */
    private static function parents(array $body): ?string
    {
        $parents = $body['parents'] ?? null;
        if ($parents === null) return null;
        if (!is_array($parents) || !array_is_list($parents) || count($parents) > 100) self::invalid('parents', 'parents is a list of generation ids.');
        foreach ($parents as $id) {
            if (!is_int($id) || $id < 1) self::invalid('parents', 'parents is a list of generation ids.');
        }
        return json_encode($parents, self::JSON_FLAGS) ?: null;
    }

    /** @param array<mixed> $body a JSON list (`inputs`) or object (`params`) of at most 64 KB */
    private static function json(array $body, string $field, bool $object): ?string
    {
        $value = $body[$field] ?? null;
        if ($value === null) return null;
        $ok = is_array($value) && ($object ? ($value === [] || !array_is_list($value)) : array_is_list($value));
        $json = $ok ? json_encode($object && $value === [] ? new \stdClass() : $value, self::JSON_FLAGS) : false;
        if ($json === false || strlen($json) > self::JSON_MAX) self::invalid($field, "$field is a JSON " . ($object ? 'object' : 'list') . ' of at most 64 KB.');
        return $json;
    }

    /** @param array<mixed> $body `work_path`: a file of the chapter's work storage, or null */
    private function workPath(string $toon, int $chapter, array $body): ?string
    {
        $name = $body['work_path'] ?? null;
        if ($name === null || $name === '') return null;
        if (!is_string($name) || !$this->work->exists($toon, $chapter, $name)) self::invalid('work_path', 'work_path is a file in the work storage of this chapter (PUT /api/storyboard/work first).');
        return $name;
    }
}
