<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Opis\JsonSchema\Errors\ErrorFormatter;
use Opis\JsonSchema\Validator;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\Expr\TextCheck;
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
        'place' => 'map-place',
        'route' => 'map-route',
        'character' => 'registry-character',
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
     * per-locale one (legacy) against the old; a neutral chapter / scene's texts also by their expressions (syntax
     * errors and the old `{x:clock}`, Expr\TextCheck – agents/EXPRESSIONS.md §4)
     * @return list<string>
     */
    public function problemsOf(EntityKey $key, mixed $content, int $limit = 20): array
    {
        $problems = $this->problems($key->kind, $content, $limit, $key->isNeutralDoc());
        if ($problems || !$key->isNeutralDoc() || !in_array($key->kind, ['chapter', 'scene'], true)) return $problems;
        return TextCheck::problems($key->kind, $content, $limit);
    }

    /** Throws 422 invalid_content { problems } when $content doesn't fit its kind (or a text's expression is broken) */
    public function guard(EntityKey $key, mixed $content): void
    {
        // the entries of the world map / the character registry are documents of their own (agents/MAP-DOCUMENTS.md): an
        // older client that saves the whole collection into its anchor would be ignored silently – refused instead
        $collection = CollectionDocs::ofAnchor($key->kind);
        if ($collection !== null && $content instanceof \stdClass) {
            foreach ($collection['parts'] as $kind => $field) {
                if (property_exists($content, $field)) {
                    throw new ApiError(422, 'invalid_content', "The $key->kind document holds only its own fields: every entry of `$field` is a document $kind:<id> of its own.",
                        ['problems' => ["/$field: the entries are documents of their own ($kind:<id>)"]]);
                }
            }
        }
        $problems = $this->problemsOf($key, $content);
        if ($problems) {
            $what = str_contains($problems[0], '(expr:') ? "has a text that can't be shown" : "doesn't fit its type";
            throw new ApiError(422, 'invalid_content', "The $key->kind document $what: " . $problems[0]
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
