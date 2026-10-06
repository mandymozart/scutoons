<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Opis\JsonSchema\Errors\ErrorFormatter;
use Opis\JsonSchema\Validator;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The type guard of the content store: every saved document is checked against the JSON Schema of its kind, generated
 * from @scutoons/content's TypeScript types (`npm run schema -w @scutoons/content` writes shared/content/schema/ and the
 * copy the server ships in server/schema/content/; `npm run check` fails when either is stale).
 * Checked with opis/json-schema (draft-07). The schemas allow extra fields (the generator's `additionalProperties: true`), so they guard shapes and types, not
 * unknown keys. `locations` / `routes` (the old per-toon map, read-only) have no schema and are not checked.
 */
final class ContentSchemas
{
    /** entity kind → schema file (server/schema/content/<name>.schema.json); null = not checked */
    public const BY_KIND = [
        'toon' => 'toon-index',
        'toon-settings' => 'toon',
        'season' => 'season',
        'chapter' => 'chapter',
        'scene' => 'group',
        'locations' => null,
        'routes' => null,
        'cast' => 'cast',
        'credits' => 'credits',
        'script' => 'script',
        'characters' => 'characters',
        'moods' => 'moods',
        'asset-requests' => 'asset-requests',
        'people' => 'people',
        'roles' => 'roles',
        'events' => 'events',
        'map' => 'map',
    ];

    /** the language-neutral documents of these kinds (a key without locale, agents/NEUTRAL.md) → schema file */
    public const NEUTRAL_BY_KIND = [
        'chapter' => 'chapter-doc',
        'scene' => 'group-doc',
        'season' => 'season-doc',
    ];

    private ?Validator $validator = null;

    public function __construct(private readonly Settings $settings)
    {
    }

    /**
     * Problems of $content (decoded with stdClass objects) as a document of $kind; [] = valid or not checked
     * @return list<string>
     */
    public function problems(string $kind, mixed $content, int $limit = 20, bool $neutral = false): array
    {
        // EntityKey only parses known kinds; a stray older kind isn't checked
        $name = $neutral && isset(self::NEUTRAL_BY_KIND[$kind]) ? self::NEUTRAL_BY_KIND[$kind] : (self::BY_KIND[$kind] ?? null);
        if ($name === null) return [];
        $result = $this->validator($limit)->validate($content, "https://scutoons.com/schema/$name.schema.json");
        $error = $result->error();
        if ($error === null) return [];
        $problems = [];
        foreach ((new ErrorFormatter())->format($error, true) as $path => $messages) {
            foreach ($messages as $message) $problems[] = ($path === '/' ? '(document)' : $path) . ": $message";
        }
        return array_slice($problems, 0, $limit);
    }

    /**
     * Problems of a stored document by its key: a neutral chapter / scene / season against its neutral schema, a
     * per-locale one (legacy) against the old
     * @return list<string>
     */
    public function problemsOf(EntityKey $key, mixed $content, int $limit = 20): array
    {
        return $this->problems($key->kind, $content, $limit, $key->isNeutralDoc());
    }

    /** Throws 422 invalid_content { problems } when $content doesn't fit its kind */
    public function guard(EntityKey $key, mixed $content): void
    {
        $problems = $this->problemsOf($key, $content);
        if ($problems) {
            throw new ApiError(422, 'invalid_content', "The $key->kind document doesn't fit its type: " . $problems[0]
                . (count($problems) > 1 ? ' (+' . (count($problems) - 1) . ' more)' : ''), ['problems' => $problems]);
        }
    }

    private function validator(int $limit): Validator
    {
        if ($this->validator === null) {
            $this->validator = new Validator();
            $this->validator->resolver()?->registerPrefix('https://scutoons.com/schema/', $this->settings->root() . '/schema/content/');
        }
        $this->validator->setMaxErrors($limit);
        return $this->validator;
    }
}
