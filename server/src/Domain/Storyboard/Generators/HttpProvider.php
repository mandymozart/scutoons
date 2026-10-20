<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard\Generators;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Config\Settings;

/**
 * What the HTTP providers share: the key and the API base from the settings (`<X>_API_KEY`, `<X>_API_URL` – the tests
 * point the URL at a stand-in), one JSON call, the checks of a job.
 */
abstract class HttpProvider implements Generator
{
    public function __construct(protected readonly Settings $settings, protected readonly HttpClient $http)
    {
    }

    abstract protected function urlSetting(): string;

    public function configured(): bool
    {
        return trim($this->settings->get($this->keySetting())) !== '';
    }

    public function unavailable(): ?string
    {
        return null;
    }

    protected function base(): string
    {
        return rtrim($this->settings->get($this->urlSetting()), '/');
    }

    /**
     * One JSON call → the decoded answer; ProviderFailure on no answer (status 0) or a status ≥ 400
     * @param array<string, string> $headers
     * @param array<mixed>|null $body
     * @return array<mixed>
     */
    protected function call(string $method, string $path, array $headers, ?array $body = null): array
    {
        if (!$this->configured()) throw new ApiError(409, 'generator_not_configured', "{$this->label()} has no key on this server ({$this->keySetting()}).", ['generator' => $this->id(), 'setting' => $this->keySetting()]);
        $headers += ['Accept' => 'application/json'];
        if ($body !== null) $headers['Content-Type'] = 'application/json';
        try {
            $answer = $this->http->send($method, $this->base() . $path, $headers, $body !== null ? (string) json_encode($body, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE) : null);
        } catch (RuntimeException $e) {
            throw new ProviderFailure("{$this->label()} did not answer: {$e->getMessage()}", 0);
        }
        $json = json_decode($answer['body'], true);
        if ($answer['status'] === 0) throw new ProviderFailure("{$this->label()} did not answer.", 0);
        if ($answer['status'] >= 400) {
            throw new ProviderFailure("{$this->label()} {$answer['status']}: " . $this->why($answer['status'], is_array($json) ? $json : [], $answer['body']), $answer['status']);
        }
        return is_array($json) ? $json : [];
    }

    /**
     * The provider's reason in a few words
     * @param array<mixed> $json
     */
    protected function why(int $status, array $json, string $body): string
    {
        if ($status === 401 || $status === 403) return "the key was refused ({$this->keySetting()})";
        if ($status === 429) return 'too many requests – the provider\'s limit is reached';
        foreach (['error', 'message', 'detail', 'failure_reason'] as $field) {
            $value = $json[$field] ?? null;
            if (is_array($value)) $value = $value['message'] ?? json_encode($value);
            if (is_string($value) && $value !== '') return mb_substr($value, 0, 300);
        }
        return mb_substr(trim(strip_tags($body)), 0, 200) ?: 'refused';
    }

    /** @return array<string, mixed> the model by id; 422 invalid_job { field: model } */
    protected function model(string $id): array
    {
        foreach ($this->models() as $model) {
            if ($model['id'] === $id) return $model;
        }
        $ids = array_map(fn(array $m) => (string) $m['id'], $this->models());
        throw self::invalid('model', "{$this->label()} has no model $id (" . implode(', ', $ids) . ').');
    }

    protected static function invalid(string $field, string $message): ApiError
    {
        return new ApiError(422, 'invalid_job', $message, ['field' => $field]);
    }

    /**
     * The ratio of $ratios nearest to $aspect ("9:16", by the log of the quotient); the first when $aspect isn't one
     * @param list<string> $ratios
     */
    public static function nearestRatio(?string $aspect, array $ratios): string
    {
        $value = static function (string $ratio): float {
            [$a, $b] = array_map('floatval', explode(':', $ratio) + [1 => '1']);
            return $a > 0 && $b > 0 ? $a / $b : 1.0;
        };
        if ($aspect === null || !preg_match('/^\d+(\.\d+)?:\d+(\.\d+)?$/', $aspect)) return $ratios[0];
        $want = $value($aspect);
        $best = $ratios[0];
        foreach ($ratios as $ratio) {
            if (abs(log($value($ratio) / $want)) < abs(log($value($best) / $want))) $best = $ratio;
        }
        return $best;
    }

    /**
     * The references of these roles
     * @param list<array{role: string, url: string, type: ?string, seconds: ?float}> $refs
     * @param list<string> $roles
     * @return list<array{role: string, url: string, type: ?string, seconds: ?float}>
     */
    protected static function refs(array $refs, array $roles): array
    {
        return array_values(array_filter($refs, fn(array $r) => in_array($r['role'], $roles, true)));
    }

    /** @param array<string, mixed> $params */
    protected static function seed(array $params): ?int
    {
        $seed = $params['seed'] ?? null;
        if ($seed === null) return null;
        if (!is_int($seed) || $seed < 0 || $seed > 4294967295) throw self::invalid('params.seed', 'seed is a whole number from 0 to 4294967295.');
        return $seed;
    }
}
