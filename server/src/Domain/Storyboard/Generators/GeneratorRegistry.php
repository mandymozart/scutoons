<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard\Generators;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\AssetStorage\CurlHttpClient;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The paid providers the server starts storyboard generations at (#121): luma-image, runway, gemini-image (listed, not
 * startable yet). One HTTP client for all (tests: useHttp – a stand-in provider, never a real paid call).
 */
final class GeneratorRegistry
{
    /** @var array<string, Generator>|null */
    private ?array $generators = null;
    private HttpClient $http;

    public function __construct(private readonly Settings $settings)
    {
        $this->http = new CurlHttpClient(30);
    }

    /** tests: a stand-in for the providers */
    public function useHttp(HttpClient $http): void
    {
        $this->http = $http;
        $this->generators = null;
    }

    public function http(): HttpClient
    {
        return $this->http;
    }

    /** @return array<string, Generator> */
    public function all(): array
    {
        return $this->generators ??= array_column(array_map(fn(Generator $g) => ['id' => $g->id(), 'g' => $g], [
            new LumaImage($this->settings, $this->http),
            new Runway($this->settings, $this->http),
            new GeminiImage($this->settings, $this->http),
        ]), 'g', 'id');
    }

    /** 422 unknown_generator */
    public function get(mixed $id): Generator
    {
        $all = $this->all();
        if (!is_string($id) || !isset($all[$id])) throw new ApiError(422, 'unknown_generator', 'Unknown generator (' . implode(', ', array_keys($all)) . ').', ['field' => 'generator']);
        return $all[$id];
    }

    /**
     * What GET /storyboard/generators lists: id, label, configured (+ the key's setting name – never its value), why
     * it can't run, the models with their prices
     * @return list<array<string, mixed>>
     */
    public function describe(): array
    {
        return array_values(array_map(fn(Generator $g) => [
            'id' => $g->id(), 'label' => $g->label(), 'configured' => $g->configured(), 'setting' => $g->keySetting(), 'paid' => true,
            'unavailable' => $g->unavailable(), 'kinds' => array_values(array_unique(array_map(fn(array $m) => (string) $m['kind'], $g->models()))), 'models' => $g->models(),
        ], $this->all()));
    }
}
