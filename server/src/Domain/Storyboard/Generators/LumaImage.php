<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard\Generators;

/**
 * Luma's image model uni-1 / uni-1-max through the Luma Agents API (what scripts/storyboard/comfy.mjs `--engine luma`
 * ran for the owner; docs.agents.lumalabs.ai, read 2026-10-21): `POST /v1/generations` { prompt, model, type: image,
 * aspect_ratio (one of nine), style: auto | manga, output_format, image_ref: [{ url }] (≤ 9) } – or type `image_edit`
 * with the shot's still as `source: { url }` (+ ≤ 8 image_ref) when a reference has the role `still`. Then
 * `GET /v1/generations/{id}`: state queued → processing → completed (output[{ type: image, url }] – a presigned link
 * that expires after 1 hour) | failed (failure_code, failure_reason). No cancel endpoint. Bearer key LUMA_API_KEY.
 * Price per image in USD (the pricing page, 2026-10-21; the API reports no cost, so the estimate is booked): text → image
 * uni-1 0.0404 / max 0.1000; with 1 input image (a reference or the edited still) 0.0434 / 0.1030, each more + 0.0030.
 * Failures with content_moderated / generation_failed / output_not_found are refunded (cost 0).
 */
final class LumaImage extends HttpProvider
{
    public const RATIOS = ['3:1', '2:1', '16:9', '3:2', '1:1', '2:3', '9:16', '1:2', '1:3'];
    /** the ratios style `manga` takes */
    public const PORTRAIT = ['2:3', '9:16', '1:2', '1:3'];
    public const REFUNDED = ['content_moderated', 'generation_failed', 'output_not_found'];
    private const MAX_INPUTS = 9;
    private const PER_MORE = 0.0030;

    public function id(): string
    {
        return 'luma-image';
    }

    public function label(): string
    {
        return 'Luma (uni-1)';
    }

    public function keySetting(): string
    {
        return 'LUMA_API_KEY';
    }

    protected function urlSetting(): string
    {
        return 'LUMA_API_URL';
    }

    public function models(): array
    {
        $refs = ['roles' => ['still', 'reference', 'character', 'style'], 'max' => self::MAX_INPUTS, 'note' => 'a `still` is edited (image_edit, then ≤ 8 more)'];
        return [
            ['id' => 'uni-1', 'label' => 'uni-1 – image', 'kind' => 'image', 'mode' => 'image', 'refs' => $refs, 'ratios' => self::RATIOS, 'styles' => ['auto', 'manga'],
                'price' => ['currency' => 'USD', 'text' => 0.0404, 'one_image' => 0.0434, 'per_more_image' => self::PER_MORE]],
            ['id' => 'uni-1-max', 'label' => 'uni-1-max – image (higher quality)', 'kind' => 'image', 'mode' => 'image', 'refs' => $refs, 'ratios' => self::RATIOS, 'styles' => ['auto', 'manga'],
                'price' => ['currency' => 'USD', 'text' => 0.1000, 'one_image' => 0.1030, 'per_more_image' => self::PER_MORE]],
        ];
    }

    public function estimate(array $job): array
    {
        $model = $this->model($job['model']);
        if (trim((string) $job['prompt']) === '') throw self::invalid('prompt', 'Luma needs a prompt.');
        $inputs = count($job['refs']);
        if ($inputs > self::MAX_INPUTS) throw self::invalid('references', 'Luma takes at most ' . self::MAX_INPUTS . ' images (the edited still counts).');
        if (count(self::refs($job['refs'], ['still'])) > 1) throw self::invalid('references', 'One still to edit at most.');
        foreach ($job['refs'] as $ref) {
            if ($ref['type'] !== null && !str_starts_with($ref['type'], 'image/')) throw self::invalid('references', 'Luma\'s image model takes images only.');
        }
        $params = $job['params'];
        $ratio = $params['ratio'] ?? null;
        if ($ratio !== null && !in_array($ratio, self::RATIOS, true)) throw self::invalid('params.ratio', 'ratio is one of ' . implode(', ', self::RATIOS) . '.');
        $ratio ??= self::nearestRatio($job['aspect'], self::RATIOS);
        $style = $params['style'] ?? 'auto';
        if (!in_array($style, ['auto', 'manga'], true)) throw self::invalid('params.style', 'style is auto or manga.');
        if ($style === 'manga' && !in_array($ratio, self::PORTRAIT, true)) throw self::invalid('params.style', 'manga takes a portrait ratio (' . implode(', ', self::PORTRAIT) . ').');
        $format = $params['output_format'] ?? 'png';
        if (!in_array($format, ['png', 'jpeg'], true)) throw self::invalid('params.output_format', 'output_format is png or jpeg.');
        $price = $model['price'];
        $amount = $inputs === 0 ? $price['text'] : $price['one_image'] + self::PER_MORE * ($inputs - 1);
        $edit = self::refs($job['refs'], ['still']) !== [];
        return ['amount' => round($amount, 4), 'currency' => 'USD', 'kind' => 'image', 'mode' => $edit ? 'image_edit' : 'image', 'model' => $model['id'], 'ratio' => $ratio,
            'duration' => null, 'seed' => null, 'note' => "$inputs input image" . ($inputs === 1 ? '' : 's') . ", style $style"];
    }

    public function start(array $job, array $estimate): array
    {
        $params = $job['params'];
        $source = self::refs($job['refs'], ['still'])[0] ?? null;
        $refs = array_map(fn(array $r) => ['url' => $r['url']], self::refs($job['refs'], ['reference', 'character', 'style', 'keyframe', 'control']));
        $body = ['prompt' => (string) $job['prompt'], 'model' => $estimate['model'], 'type' => $estimate['mode'], 'aspect_ratio' => $estimate['ratio'],
            'style' => $params['style'] ?? 'auto', 'output_format' => $params['output_format'] ?? 'png'];
        if ($source !== null) $body['source'] = ['url' => $source['url']];
        if ($refs !== []) $body['image_ref'] = $refs;
        $created = $this->call('POST', '/v1/generations', $this->headers(), $body);
        $id = $created['id'] ?? null;
        if (!is_string($id) || $id === '') throw new ProviderFailure('Luma answered without a generation id.', 200);
        return ['job' => $id, 'cost' => null];
    }

    public function status(string $job, array $row): array
    {
        $j = $this->call('GET', '/v1/generations/' . rawurlencode($job), $this->headers());
        $state = $j['state'] ?? null;
        if ($state === 'completed') {
            foreach (is_array($j['output'] ?? null) ? $j['output'] : [] as $out) {
                if (is_array($out) && ($out['type'] ?? 'image') === 'image' && is_string($out['url'] ?? null)) return ['state' => 'succeeded', 'progress' => 1.0, 'url' => $out['url'], 'cost' => null];
            }
            return ['state' => 'failed', 'error' => 'Luma: completed without an image', 'cost' => 0.0];
        }
        if ($state === 'failed') {
            $code = is_string($j['failure_code'] ?? null) ? $j['failure_code'] : '';
            $reason = is_string($j['failure_reason'] ?? null) ? $j['failure_reason'] : '';
            return ['state' => 'failed', 'error' => mb_substr(trim("Luma: $code $reason"), 0, 1000), 'cost' => in_array($code, self::REFUNDED, true) ? 0.0 : null];
        }
        return ['state' => $state === 'processing' ? 'running' : 'queued'];
    }

    public function cancel(string $job): bool
    {
        return false;
    }

    /** @return array<string, string> */
    private function headers(): array
    {
        return ['Authorization' => 'Bearer ' . trim($this->settings->get('LUMA_API_KEY'))];
    }
}
