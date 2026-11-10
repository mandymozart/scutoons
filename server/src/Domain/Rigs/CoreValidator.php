<?php
declare(strict_types=1);

namespace Scutoons\Domain\Rigs;

use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Config\Settings;

/**
 * Instruments, presets and rigs are checked by the Rust core (#313, owner 2026-11-02: "the rigs are also fundamentally
 * rust modules running with the core and are universal"): the API keeps no rule of its own – it sends the document with
 * what it names, by value (a code rig's policy module as its descriptor, #414), to the build service's `POST /validate` (deck/build; deck-space rig/validate.rs, the code the
 * decks play with) and returns the core's problems unchanged. The service is the module compiler's (`MODULE_BUILD_URL`,
 * the shared secret `MODULE_BUILD_TOKEN` as `X-Build-Token`); without it nothing of these kinds reaches the catalog –
 * 503, never a silent pass. Drafts are saved without it (CatalogDocs::guardSave).
 */
final class CoreValidator
{
    public function __construct(private readonly Settings $settings, private readonly HttpClient $http)
    {
    }

    /**
     * The core's problems of $document as catalog entry $id of $kind (none = valid)
     * @param 'instruments'|'presets'|'rigs' $kind
     * @param list<mixed> $instruments the instruments it names, decoded with objects
     * @param list<mixed> $presets the presets it names and their ancestors (without itself), decoded with objects
     * @param list<mixed> $modules a code rig's policy module (#414): the descriptor of its `module:<id>` document, decoded with
     *        objects; sent as `modules` only when there is one (a declarative rig's request stays what #313 fixed)
     * @return list<string>
     */
    public function problems(string $kind, string $id, mixed $document, array $instruments, array $presets, array $modules = []): array
    {
        $url = rtrim($this->settings->get('MODULE_BUILD_URL'), '/');
        $token = $this->settings->get('MODULE_BUILD_TOKEN');
        if ($url === '' || $token === '') throw new ApiError(503, 'no_build_service', 'The build service that checks instruments, presets and rigs is not set up.');
        $request = ['kind' => $kind, 'id' => $id, 'document' => $document, 'instruments' => $instruments, 'presets' => $presets];
        if ($modules !== []) $request['modules'] = $modules;
        return $this->ask($url, $token, $request, "$kind $id");
    }

    /**
     * #90: the core's problems of a show and the whole tree under it (deck-show's `validate` and `resolve`)
     * @param list<mixed> $shows every show the tree names, by value, decoded with objects
     * @return list<string>
     */
    public function showProblems(string $id, mixed $document, array $shows): array
    {
        $url = rtrim($this->settings->get('MODULE_BUILD_URL'), '/');
        $token = $this->settings->get('MODULE_BUILD_TOKEN');
        if ($url === '' || $token === '') throw new ApiError(503, 'no_build_service', 'The build service that checks shows is not set up.');
        return $this->ask($url, $token, ['kind' => 'shows', 'id' => $id, 'document' => $document, 'shows' => $shows], "show $id");
    }

    /**
     * @param array<string, mixed> $request
     * @return list<string>
     */
    private function ask(string $url, string $token, array $request, string $what): array
    {
        $body = (string) json_encode($request, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION);
        try {
            $response = $this->http->send('POST', "$url/validate", ['Content-Type' => 'application/json', 'X-Build-Token' => $token], $body);
        } catch (RuntimeException) {
            throw new ApiError(503, 'build_unavailable', 'The build service that checks instruments, presets and rigs does not answer – nothing was saved.');
        }
        $answer = json_decode($response['body'], true);
        $problems = is_array($answer) ? ($answer['problems'] ?? null) : null;
        if ($response['status'] === 200 && is_array($problems) && array_is_list($problems) && array_filter($problems, fn($p) => !is_string($p)) === []) {
            /** @var list<string> $problems */
            return $problems;
        }
        if ($response['status'] === 503) throw new ApiError(503, 'build_unavailable', 'The build service that checks instruments, presets and rigs is not available – nothing was saved.');
        throw new ApiError(502, 'build_failed', 'The build service answered ' . $response['status'] . ' to a check of ' . $what . '.');
    }
}
