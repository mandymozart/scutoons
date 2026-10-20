<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard\Generators;

/**
 * Runway's developer API (ported from workflows/storyboard-studio/generators/runway.mjs; docs.dev.runwayml.com, API
 * version 2024-11-06, read 2026-10-19):
 *   POST /v1/image_to_video  gen4_turbo (5 credits/s) | gen4.5 (12 credits/s, needs a prompt): the shot's still as the
 *                            first frame (`promptImage`, a URL) + the prompt, 2–10 s, the ratio nearest to the shot's
 *   POST /v1/video_to_video  aleph2 (28 credits/s, at least 56): a control video (a handheld pass, ≤ 30 s, `videoUri`)
 *                            restyled by the prompt, the still as the keyframe at 0; the output follows the input
 * then GET /v1/tasks/{id} (PENDING · THROTTLED · RUNNING + progress · SUCCEEDED + output URLs (24–48 h) · FAILED ·
 * CANCELLED), DELETE /v1/tasks/{id} cancels. 1 credit = 0.01 USD. Bearer key RUNWAY_API_KEY, header X-Runway-Version.
 */
final class Runway extends HttpProvider
{
    public const VERSION = '2024-11-06';
    public const CREDIT_USD = 0.01;
    public const I2V_RATIOS = ['1280:720', '720:1280', '1104:832', '960:960', '832:1104', '1584:672'];

    public function id(): string
    {
        return 'runway';
    }

    public function label(): string
    {
        return 'Runway';
    }

    public function keySetting(): string
    {
        return 'RUNWAY_API_KEY';
    }

    protected function urlSetting(): string
    {
        return 'RUNWAY_API_URL';
    }

    public function models(): array
    {
        return [
            ['id' => 'gen4_turbo', 'label' => 'Gen-4 Turbo – image to video', 'kind' => 'video', 'mode' => 'image_to_video', 'refs' => ['roles' => ['still'], 'max' => 1, 'required' => ['still']],
                'durations' => ['min' => 2, 'max' => 10], 'ratios' => self::I2V_RATIOS, 'price' => ['currency' => 'USD', 'credits_per_second' => 5, 'credit' => self::CREDIT_USD]],
            ['id' => 'gen4.5', 'label' => 'Gen-4.5 – image to video', 'kind' => 'video', 'mode' => 'image_to_video', 'refs' => ['roles' => ['still'], 'max' => 1, 'required' => ['still']], 'prompt_required' => true,
                'durations' => ['min' => 2, 'max' => 10], 'ratios' => self::I2V_RATIOS, 'price' => ['currency' => 'USD', 'credits_per_second' => 12, 'credit' => self::CREDIT_USD]],
            ['id' => 'aleph2', 'label' => 'Aleph 2 – video to video (the handheld pass)', 'kind' => 'video', 'mode' => 'video_to_video', 'refs' => ['roles' => ['control', 'still'], 'max' => 2, 'required' => ['control']],
                'max_input_seconds' => 30, 'price' => ['currency' => 'USD', 'credits_per_second' => 28, 'min_credits' => 56, 'credit' => self::CREDIT_USD]],
        ];
    }

    public function estimate(array $job): array
    {
        $model = $this->model($job['model']);
        $prompt = trim((string) $job['prompt']);
        if (($model['prompt_required'] ?? false) && $prompt === '') throw self::invalid('prompt', "{$model['id']} needs a prompt.");
        $stills = self::refs($job['refs'], ['still']);
        $controls = self::refs($job['refs'], ['control']);
        if (count($stills) > 1 || count($controls) > 1) throw self::invalid('references', 'One still and one control video at most.');
        foreach ($job['refs'] as $ref) {
            if (!in_array($ref['role'], $model['refs']['roles'], true)) throw self::invalid('references', "{$model['id']} takes " . implode(' + ', $model['refs']['roles']) . " – not a {$ref['role']}.");
        }
        $params = $job['params'];
        $ratio = null;
        if ($model['mode'] === 'video_to_video') {
            $control = $controls[0] ?? throw self::invalid('references', 'Aleph needs the control video (a handheld pass or a generated clip, an MP4).');
            if ($control['type'] !== null && $control['type'] !== 'video/mp4') throw self::invalid('references', 'Aleph takes an MP4.');
            $seconds = $control['seconds'] ?? (is_int($params['duration'] ?? null) || is_float($params['duration'] ?? null) ? (float) $params['duration'] : null);
            if ($seconds === null || $seconds <= 0) throw self::invalid('params.duration', 'The control video\'s length is unknown – give params.duration (its seconds).');
            if ($seconds > $model['max_input_seconds']) throw self::invalid('references', sprintf('The video is %.1f s – Runway takes at most %d s.', $seconds, $model['max_input_seconds']));
            $note = 'the output follows the input video (length, aspect)';
        } else {
            if ($stills === []) throw self::invalid('references', 'Pick the still first (the first frame).');
            $duration = $params['duration'] ?? 5;
            if (!is_int($duration) && !is_float($duration)) throw self::invalid('params.duration', 'duration is seconds.');
            $seconds = (float) min($model['durations']['max'], max($model['durations']['min'], (int) round($duration)));
            $given = $params['ratio'] ?? null;
            if ($given !== null && !in_array($given, self::I2V_RATIOS, true)) throw self::invalid('params.ratio', 'ratio is one of ' . implode(', ', self::I2V_RATIOS) . '.');
            $ratio = $given ?? self::nearestRatio($job['aspect'], self::I2V_RATIOS);
            $note = null;
        }
        $credits = (int) ceil(max($model['price']['min_credits'] ?? 0, $model['price']['credits_per_second'] * $seconds));
        return ['amount' => round($credits * self::CREDIT_USD, 4), 'currency' => 'USD', 'kind' => 'video', 'mode' => $model['mode'], 'model' => $model['id'], 'ratio' => $ratio,
            'duration' => $seconds, 'seed' => self::seed($params), 'note' => "$credits credits" . ($note !== null ? " – $note" : '')];
    }

    public function start(array $job, array $estimate): array
    {
        $prompt = mb_substr(trim((string) $job['prompt']), 0, 1000);
        $still = self::refs($job['refs'], ['still'])[0] ?? null;
        $optional = ($prompt !== '' ? ['promptText' => $prompt] : []) + ($estimate['seed'] !== null ? ['seed' => $estimate['seed']] : []);
        if ($estimate['mode'] === 'image_to_video') {
            $body = ['model' => $estimate['model'], 'promptImage' => $still['url'] ?? '', 'ratio' => $estimate['ratio'], 'duration' => (int) $estimate['duration']] + $optional;
        } else {
            $control = self::refs($job['refs'], ['control'])[0];
            $body = ['model' => $estimate['model'], 'videoUri' => $control['url']] + $optional + ($still !== null ? ['keyframes' => [['uri' => $still['url'], 'at' => 0]]] : []);
        }
        $created = $this->call('POST', '/v1/' . $estimate['mode'], $this->headers(), $body);
        $id = $created['id'] ?? null;
        if (!is_string($id) || $id === '') throw new ProviderFailure('Runway answered without a task id.', 200);
        return ['job' => $id, 'cost' => null];
    }

    public function status(string $job, array $row): array
    {
        $t = $this->call('GET', '/v1/tasks/' . rawurlencode($job), $this->headers());
        $credits = $t['cost']['credits'] ?? null;
        $cost = is_int($credits) || is_float($credits) ? round($credits * self::CREDIT_USD, 4) : null;
        $progress = is_int($t['progress'] ?? null) || is_float($t['progress'] ?? null) ? max(0.0, min(1.0, (float) $t['progress'])) : null;
        return match ($t['status'] ?? null) {
            'SUCCEEDED' => is_string($t['output'][0] ?? null)
                ? ['state' => 'succeeded', 'progress' => 1.0, 'url' => $t['output'][0], 'cost' => $cost]
                : ['state' => 'failed', 'error' => 'Runway: succeeded without an output', 'cost' => $cost],
            'FAILED' => ['state' => 'failed', 'error' => mb_substr('Runway: ' . (is_string($t['failure'] ?? null) ? $t['failure'] : 'failed') . (is_string($t['failureCode'] ?? null) ? " ({$t['failureCode']})" : ''), 0, 1000), 'cost' => $cost],
            'CANCELLED' => ['state' => 'cancelled', 'cost' => $cost],
            'RUNNING' => ['state' => 'running', 'progress' => $progress],
            default => ['state' => 'queued'],
        };
    }

    public function cancel(string $job): bool
    {
        $this->call('DELETE', '/v1/tasks/' . rawurlencode($job), $this->headers());
        return true;
    }

    /** @return array<string, string> */
    private function headers(): array
    {
        return ['Authorization' => 'Bearer ' . trim($this->settings->get('RUNWAY_API_KEY')), 'X-Runway-Version' => self::VERSION];
    }
}
