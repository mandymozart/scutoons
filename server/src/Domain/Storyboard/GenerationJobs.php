<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Storyboard\Generators\Generator;
use Scutoons\Domain\Storyboard\Generators\GeneratorRegistry;
use Scutoons\Domain\Storyboard\Generators\ProviderFailure;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * Paid provider jobs of the storyboard (#121, agents/research/STUDIO-IN-CREATOR.md §2.5) – without a worker: start()
 * checks the budget (GenerationBudget), reserves the estimate (the row, status queued), starts the provider's task once
 * and stores its id; reading a job (refresh) asks the provider at most once per GENERATION_POLL_SECONDS and books the
 * result link + the actual cost; the result itself is passed through (result()) – the server stores no file. Members
 * and their AI connections (owner 2026-10-20: "may start paid jobs") – every row says who (created_by, via_token).
 */
final class GenerationJobs
{
    public const ROLES = ['still', 'control', 'keyframe', 'reference', 'character', 'style'];
    /** the fields a job (POST) takes */
    private const FIELDS = ['toon', 'chapter', 'shot', 'generator', 'model', 'prompt', 'references', 'params', 'aspect', 'take', 'parents', 'note'];
    private const MAX_REFS = 12;
    /** at most this many open jobs are asked about in one list */
    private const LIST_REFRESH = 10;
    private const FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly Generations $generations,
        private readonly GeneratorRegistry $registry,
        private readonly GenerationBudget $budget,
        private readonly WorkStorage $work,
        private readonly RefLinks $links,
    ) {
    }

    /**
     * A POST body is a provider job (not a record of a finished generation): its generator is one of the registry's
     * and it names no file, machine or status
     * @param array<mixed> $body
     */
    public function isJob(array $body): bool
    {
        return is_string($body['generator'] ?? null) && isset($this->registry->all()[$body['generator']])
            && !array_key_exists('file', $body) && !array_key_exists('machine', $body) && !array_key_exists('status', $body);
    }

    /**
     * The estimate of a job without starting it (nothing stored, no budget taken)
     * @param array<mixed> $body
     * @return array<string, mixed>
     */
    public function estimate(array $body): array
    {
        [$generator, $job] = $this->prepare($body);
        $estimate = $generator->estimate($job);
        return $estimate + ['generator' => $generator->id(), 'budget' => $this->budget->state()];
    }

    /**
     * Starts a paid job → the stored row (status queued, `job` = the provider's id). 402 no_budget / budget_exhausted,
     * 409 generator_not_configured, 422 invalid_job / unknown_generator / generator_unavailable / invalid_generation,
     * 502 provider_failed { item } (the row stays as failed: refused = cost 0, no answer = the estimate booked – it may
     * have been billed)
     * @param array<string, mixed> $user
     * @param array{id: string, name: string}|null $token
     * @param array<mixed> $body
     * @return array<string, mixed>
     */
    public function start(array $user, ?array $token, array $body): array
    {
        $this->generations->requireAvailable();
        [$generator, $job, $row] = $this->prepare($body);
        $estimate = $generator->estimate($job);
        $now = $this->clock->utc();
        $values = $row + [
            'kind' => $estimate['kind'], 'generator' => $generator->id(), 'model' => $estimate['model'], 'mode' => $estimate['mode'], 'job' => null, 'status' => 'queued',
            'prompt' => $job['prompt'], 'seed' => $estimate['seed'], 'ratio' => $estimate['ratio'] !== null && preg_match('#^[1-9][0-9]{0,3}:[1-9][0-9]{0,3}$#', $estimate['ratio']) ? $estimate['ratio'] : null,
            'duration' => $estimate['duration'], 'estimate' => $estimate['amount'], 'currency' => $estimate['currency'],
            'created_by' => (string) $user['id'], 'via_token' => $token['id'] ?? null, 'created_at' => $now, 'updated_at' => $now,
        ];
        $id = $this->budget->reserve($estimate['amount'], $estimate['currency'], function () use ($values): int {
            $columns = array_keys($values);
            $this->db->run('INSERT INTO storyboard_generations (' . implode(', ', array_map(fn($c) => "`$c`", $columns)) . ') VALUES ('
                . implode(', ', array_fill(0, count($columns), '?')) . ')', array_values($values));
            return $this->db->lastInsertId();
        });
        try {
            $started = $generator->start($job, $estimate);
        } catch (ProviderFailure $e) {
            $billed = !$e->answered();
            $this->set($id, ['status' => 'failed', 'cost' => $billed ? $estimate['amount'] : 0.0,
                'error' => mb_substr($e->getMessage() . ($billed ? ' – the start may have been billed; the estimate is booked (check the provider\'s dashboard)' : ''), 0, 1000)]);
            throw new ApiError(502, 'provider_failed', $e->getMessage(), ['item' => $this->generations->get($id)]);
        } catch (Throwable $e) {
            $this->set($id, ['status' => 'failed', 'cost' => $estimate['amount'], 'error' => mb_substr('The start broke off: ' . $e->getMessage(), 0, 1000)]);
            throw $e;
        }
        $this->set($id, ['job' => mb_substr($started['job'], 0, 191)] + ($started['cost'] !== null ? ['cost' => $started['cost']] : []));
        return $this->generations->get($id);
    }

    /**
     * Checks the body, resolves the references to URLs the provider can fetch
     * @param array<mixed> $body
     * @return array{0: Generator, 1: array{model: string, prompt: ?string, refs: list<array{role: string, url: string, type: ?string, seconds: ?float}>, aspect: ?string, params: array<string, mixed>}, 2: array<string, mixed>}
     */
    private function prepare(array $body): array
    {
        foreach (array_keys($body) as $field) {
            if (!in_array($field, self::FIELDS, true)) self::invalid((string) $field, "Unknown field $field (a provider job takes " . implode(', ', self::FIELDS) . ').');
        }
        $generator = $this->registry->get($body['generator'] ?? null);
        if ($generator->unavailable() !== null) throw new ApiError(422, 'generator_unavailable', (string) $generator->unavailable(), ['generator' => $generator->id()]);
        if (!$generator->configured()) {
            throw new ApiError(409, 'generator_not_configured', "{$generator->label()} has no key on this server – the owner puts {$generator->keySetting()} into the server's configuration.",
                ['generator' => $generator->id(), 'setting' => $generator->keySetting()]);
        }
        [$toon, $chapter] = WorkStorage::check($body['toon'] ?? null, $body['chapter'] ?? null);
        $shot = $body['shot'] ?? null;
        if (!is_string($shot) || !preg_match('#^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$#', $shot)) self::invalid('shot', 'shot is the shot id (letters, digits, . _ -).');
        $model = $body['model'] ?? null;
        if (!is_string($model) || $model === '') self::invalid('model', 'model is one of the generator\'s models (GET /storyboard/generators).');
        $prompt = $body['prompt'] ?? null;
        if ($prompt !== null && (!is_string($prompt) || mb_strlen($prompt) > 20000)) self::invalid('prompt', 'prompt is a text of at most 20000 characters.');
        $params = $body['params'] ?? [];
        if (!is_array($params) || ($params !== [] && array_is_list($params))) self::invalid('params', 'params is a JSON object.');
        $aspect = $body['aspect'] ?? null;
        if ($aspect !== null && (!is_string($aspect) || !preg_match('#^\d{1,4}(\.\d+)?:\d{1,4}(\.\d+)?$#', $aspect))) self::invalid('aspect', 'aspect is the shot\'s, e.g. 9:16.');
        [$refs, $inputs] = $this->references($toon, $chapter, $body['references'] ?? []);
        $take = $body['take'] ?? null;
        if ($take !== null && (!is_string($take) || mb_strlen($take) > 64)) self::invalid('take', 'take is a text of at most 64 characters.');
        $note = $body['note'] ?? null;
        if ($note !== null && (!is_string($note) || mb_strlen($note) > 1000)) self::invalid('note', 'note is a text of at most 1000 characters.');
        $parents = $body['parents'] ?? null;
        if ($parents !== null && (!is_array($parents) || !array_is_list($parents) || count($parents) > 100 || array_filter($parents, fn($p) => !is_int($p) || $p < 1) !== [])) {
            self::invalid('parents', 'parents is a list of generation ids.');
        }
        /** @var array<string, mixed> $params */
        $paramsJson = json_encode(($aspect !== null ? ['aspect' => $aspect] : []) + $params ?: new \stdClass(), self::FLAGS);
        if ($paramsJson === false || strlen($paramsJson) > 65536) self::invalid('params', 'params is a JSON object of at most 64 KB.');
        $row = ['toon' => $toon, 'chapter' => $chapter, 'shot' => $shot, 'take' => $take, 'note' => $note, 'parents' => $parents !== null ? json_encode($parents) : null,
            'inputs' => $inputs !== [] ? json_encode($inputs, self::FLAGS) : null, 'params' => $paramsJson];
        return [$generator, ['model' => $model, 'prompt' => $prompt, 'refs' => $refs, 'aspect' => $aspect, 'params' => $params], $row];
    }

    /**
     * references [{ role, work: <a file of the chapter's work storage> | url: <https – a CDN asset> }] → the provider's
     * refs (a work file as a signed link) and what the row keeps (role + work / url – never the signed link)
     * @return array{0: list<array{role: string, url: string, type: ?string, seconds: ?float}>, 1: list<array<string, string>>}
     */
    private function references(string $toon, int $chapter, mixed $given): array
    {
        if (!is_array($given) || !array_is_list($given) || count($given) > self::MAX_REFS) self::invalid('references', 'references is a list of at most ' . self::MAX_REFS . ' { role, work | url }.');
        $refs = [];
        $inputs = [];
        foreach ($given as $ref) {
            $role = is_array($ref) ? ($ref['role'] ?? null) : null;
            if (!is_array($ref) || !in_array($role, self::ROLES, true)) self::invalid('references', 'A reference has a role: ' . implode(', ', self::ROLES) . '.');
            $work = $ref['work'] ?? null;
            $url = $ref['url'] ?? null;
            if (is_string($work) && $url === null) {
                if (!$this->work->exists($toon, $chapter, $work)) self::invalid('references', "$work is not in the work storage of this chapter (PUT /storyboard/work first).");
                $file = $this->work->file($toon, $chapter, $work);
                $refs[] = ['role' => (string) $role, 'url' => $this->links->link($toon, $chapter, $work), 'type' => $file['type'],
                    'seconds' => $file['type'] === 'video/mp4' ? Mp4::seconds($file['path']) : null];
                $inputs[] = ['role' => (string) $role, 'work' => $work];
            } elseif (is_string($url) && $work === null) {
                if (!preg_match('#^https://[^\s/?\#]+/\S{1,2000}$#', $url)) self::invalid('references', 'A reference url is a public https link (a CDN asset).');
                $ext = strtolower(pathinfo((string) parse_url($url, PHP_URL_PATH), PATHINFO_EXTENSION));
                $refs[] = ['role' => (string) $role, 'url' => $url, 'type' => WorkStorage::TYPES[$ext] ?? null, 'seconds' => null];
                $inputs[] = ['role' => (string) $role, 'url' => $url];
            } else {
                self::invalid('references', 'A reference names its file: work (the work storage) or url (a public https link).');
            }
        }
        return [$refs, $inputs];
    }

    /**
     * One generation, its provider job asked about first (when open and not asked about in the last few seconds)
     * @return array<string, mixed>
     */
    public function get(int $id): array
    {
        $this->refresh($id);
        return $this->generations->get($id);
    }

    /**
     * A chapter's generations, the open provider jobs (at most 10, oldest first) asked about first
     * @return list<array<string, mixed>>
     */
    public function list(mixed $toon, mixed $chapter, mixed $shot = null): array
    {
        try {
            [$t, $c] = WorkStorage::check($toon, $chapter);
            $open = $this->db->all("SELECT id FROM storyboard_generations WHERE toon = ? AND chapter = ? AND status IN ('queued', 'running') AND job IS NOT NULL ORDER BY id LIMIT " . self::LIST_REFRESH, [$t, $c]);
            foreach ($open as $row) $this->refresh((int) $row['id']);
        } catch (\PDOException) {
            // before the migration: Generations::list answers 503
        }
        return $this->generations->list($toon, $chapter, $shot);
    }

    /** Asks the provider about an open job once (rate-limited); a provider that doesn't answer leaves the row as it is */
    public function refresh(int $id): void
    {
        $row = $this->db->one('SELECT * FROM storyboard_generations WHERE id = ?', [$id]);
        if ($row === null || !in_array($row['status'], ['queued', 'running'], true) || $row['job'] === null) return;
        $every = max(1, $this->settings->int('GENERATION_POLL_SECONDS') ?: 5);
        if ($row['polled_at'] !== null && strtotime($row['polled_at'] . ' UTC') > $this->clock->time() - $every) return;
        $generator = $this->registry->all()[(string) $row['generator']] ?? null;
        if ($generator === null || !$generator->configured()) return;
        $this->set($id, ['polled_at' => $this->clock->utc()]);
        try {
            $s = $generator->status((string) $row['job'], ['estimate' => $row['estimate'] === null ? null : (float) $row['estimate'], 'status' => (string) $row['status']]);
        } catch (ProviderFailure | ApiError) {
            return;
        }
        $estimate = $row['estimate'] === null ? null : (float) $row['estimate'];
        $set = ['status' => $s['state'], 'progress' => $s['progress'] ?? null];
        if ($s['state'] === 'succeeded') {
            $set += ['result_url' => $s['url'] ?? null, 'cost' => $s['cost'] ?? $estimate, 'ms' => $this->elapsedMs((string) $row['created_at'])];
        } elseif ($s['state'] === 'failed') {
            $set += ['error' => mb_substr((string) ($s['error'] ?? 'failed'), 0, 1000), 'cost' => $s['cost'] ?? $estimate, 'ms' => $this->elapsedMs((string) $row['created_at'])];
        } elseif ($s['state'] === 'cancelled') {
            $set += ['cost' => $s['cost'] ?? ($row['status'] === 'running' ? $estimate : 0.0)];
        }
        $this->set($id, $set);
    }

    /**
     * Cancels an open provider job → the row. 409 not_open (finished already) / not_cancellable (the provider has no
     * cancel – Luma: it finishes in seconds). The cost: what the provider says, else the estimate when it was running,
     * 0 while it was queued.
     * @return array<string, mixed>
     */
    public function cancel(int $id): array
    {
        $item = $this->generations->get($id);
        if (!in_array($item['status'], ['queued', 'running'], true) || $item['job'] === null) {
            throw new ApiError(409, 'not_open', "The generation is {$item['status']} – nothing to cancel.", ['item' => $item]);
        }
        $generator = $this->registry->get($item['generator']);
        try {
            $done = $generator->cancel((string) $item['job']);
        } catch (ProviderFailure $e) {
            throw new ApiError(502, 'provider_failed', $e->getMessage(), ['item' => $item]);
        }
        if (!$done) throw new ApiError(409, 'not_cancellable', "{$generator->label()} can't cancel a job – it finishes by itself.", ['item' => $item]);
        $this->set($id, ['status' => 'cancelled', 'cost' => $item['status'] === 'running' ? $item['estimate'] : 0.0, 'progress' => null]);
        return $this->generations->get($id);
    }

    /**
     * The result of a succeeded provider job, fetched from the provider's link and handed on (for browsers the
     * provider's CORS blocks; the server keeps nothing): [stream, content type, size]. 409 no_result, 410 result_gone
     * (the provider's link ran out – Luma's after an hour), 502 provider_failed.
     * @return array{0: resource, 1: string, 2: int}
     */
    public function result(int $id): array
    {
        $item = $this->generations->get($id);
        if ($item['status'] !== 'succeeded' || !is_string($item['result_url'])) throw new ApiError(409, 'no_result', 'This generation has no provider result to pass on.');
        $tmp = tempnam(sys_get_temp_dir(), 'sb-result-');
        if ($tmp === false) throw new RuntimeException('No temp file.');
        try {
            try {
                $answer = $this->registry->http()->send('GET', $item['result_url'], [], null, null, $tmp);
            } catch (RuntimeException $e) {
                throw new ApiError(502, 'provider_failed', 'The provider did not hand out the result: ' . $e->getMessage());
            }
            if (in_array($answer['status'], [401, 403, 404, 410], true)) throw new ApiError(410, 'result_gone', 'The provider\'s link to the result ran out – download results soon after they finish.');
            if ($answer['status'] !== 200) throw new ApiError(502, 'provider_failed', "The provider answered {$answer['status']} for the result.");
            $type = strtolower(trim(explode(';', $answer['headers']['content-type'] ?? '')[0]));
            if (!preg_match('#^(image|video)/[a-z0-9.+-]+$#', $type)) $type = WorkStorage::sniff($tmp) ?? 'application/octet-stream';
            $out = fopen('php://temp/maxmemory:8388608', 'w+b');
            $in = fopen($tmp, 'rb');
            if ($out === false || $in === false) throw new RuntimeException('Cannot pass the result on.');
            $size = (int) stream_copy_to_stream($in, $out);
            fclose($in);
            rewind($out);
            return [$out, $type, $size];
        } finally {
            @unlink($tmp);
        }
    }

    private function elapsedMs(string $createdAt): int
    {
        return max(0, min(2147483647, ($this->clock->time() - (int) strtotime($createdAt . ' UTC')) * 1000));
    }

    /** @param array<string, mixed> $set */
    private function set(int $id, array $set): void
    {
        $set['updated_at'] = $this->clock->utc();
        $this->db->run('UPDATE storyboard_generations SET ' . implode(', ', array_map(fn($c) => "`$c` = ?", array_keys($set))) . ' WHERE id = ?', [...array_values($set), $id]);
    }

    private static function invalid(string $field, string $message): never
    {
        throw new ApiError(422, 'invalid_generation', $message, ['field' => $field]);
    }
}
