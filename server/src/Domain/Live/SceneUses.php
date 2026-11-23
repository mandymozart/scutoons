<?php
declare(strict_types=1);

namespace Scutoons\Domain\Live;

use Scutoons\Domain\Addresses\AddressBook;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Rigs\CoreValidator;
use stdClass;

/**
 * A use of a scene (#504, research/SCENES.md §9.2 / §9.5): `{ scene, with }` – a scene plus the fillers for its placeholders,
 * what a group, a show cell, a location or a character stores instead of a new scene document when only the fillers change
 * (owner 2026-11-12 [scenes], §9.7 Q3; its shape `SceneUse`, generated from deck-space's `Use`). Its validity is decided once,
 * on save, by the Rust core (RULES A6 [contract]): the API sends the use, its scene's published document and the toon's book
 * to the build service's `POST /validate` (kind `uses`, CoreValidator) and refuses with the core's words – it judges no filler.
 *
 * Callers: every save of a document that carries a use. None does yet – the fields come with 2·6 (SCENES.md §9.6): a reader
 * group's `scene` (chapter documents, EntitiesController's save of `chapter:<toon>:<n>`), a location's Default scene (`place:<id>`),
 * a character's Home scene (`character:<id>`) and a show cell's piece (ShowsController / Shows::save) each call `check` with
 * the document's toon (a place or character: each toon it names) before the store writes.
 */
final class SceneUses
{
    public function __construct(private readonly AddressBook $book, private readonly Scenes $scenes, private readonly CoreValidator $core)
    {
    }

    /**
     * The use if it can be stored in the toon's documents: its scene and the published version it plays; 404 for an unknown
     * toon, 422 `invalid_use { problems }` in the core's words, 503 without the build service
     * @return array{scene: string, version: int|null}
     */
    public function check(string $toon, mixed $use): array
    {
        $book = $this->book->refs($toon) ?? throw new ApiError(404, 'unknown_toon', 'No such toon.');
        $ref = $use instanceof stdClass && is_string($use->scene ?? null) ? $use->scene : '';
        $id = str_starts_with($ref, 'scene:') ? substr($ref, strlen('scene:')) : $ref;
        $scene = $id !== '' ? $this->scenes->published($id) : null;
        $problems = $this->core->useProblems($id, $use, $scene, $book, $this->scenes->modules($scene));
        if ($problems !== []) throw new ApiError(422, 'invalid_use', 'This use of the scene cannot play in this toon.', ['problems' => $problems]);
        return ['scene' => $ref, 'version' => $scene !== null && is_int($scene->version ?? null) ? $scene->version : null];
    }
}
