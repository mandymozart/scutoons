<?php
declare(strict_types=1);

namespace Scutoons\Domain\Live;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Rigs\CoreValidator;
use stdClass;

/**
 * Deck scenes (#243, research/SCENES.md §3, §9.3): `scene:<id>` in entity_versions – what a deck plays as a whole, format 2
 * (#489): `format`, `id`, `version`, `toons`, a location (a reference or a placeholder), holders (a filler or a placeholder's
 * filter, spawn zones) and the scene's own graph (deck/space/src/scene_document.rs, the one source of its shape:
 * server/schema/content/scene.schema.json and SceneTypes are generated from it; format 1 was moved by the migration
 * 2026-11-12-scene-format-2 and the schema refuses it). Like a show: the
 * server stamps `version` on every change (EntityStore::saveStamped); a draft is saved after the shape check (the JSON
 * Schema, ContentSchemas); what makes it playable is the Rust core's (deck-space over the build service's `POST /validate`,
 * kind `scenes`, as for shows and rigs): publishing asks it, a scene a deck could not load is refused with the core's words.
 * Without the service nothing is published, 503. A draft never leaves the editor: only the published version is public.
 */
final class Scenes
{
    public function __construct(private readonly EntityStore $store, private readonly CoreValidator $core)
    {
    }

    /**
     * A new draft (its shape checked by the caller: ContentSchemas, `format` included): the document names itself, the version comes from here
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function save(array $user, EntityKey $key, stdClass $body): array
    {
        return $this->store->saveStamped($user, $key, $body, 'scene');
    }

    /**
     * An older version (by its id) as the newest draft, stamped anew (a restore is a change like any save)
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function restore(array $user, EntityKey $key, stdClass $body): array
    {
        $old = isset($body->id) ? $this->store->byId((int) $body->id) : null;
        if (!$old || $old['entity_key'] !== (string) $key) throw new ApiError(404, 'not_found', 'Unknown version.');
        $restored = new stdClass();
        $restored->content = json_decode((string) $old['json']);
        $restored->message = is_string($body->message ?? null) && trim($body->message) !== '' ? $body->message : 'restore #' . $old['id'];
        return $this->save($user, $key, $restored);
    }

    /** The published document, or null */
    public function published(string $id): ?stdClass
    {
        $row = $this->store->published("scene:$id");
        $doc = $row === null ? null : json_decode((string) $row['json']);
        return $doc instanceof stdClass ? $doc : null;
    }

    /** The document at a stamped version (drafts included: the caller decides who may see one) */
    public function atVersion(string $id, int $version): ?stdClass
    {
        return $this->store->atStamp("scene:$id", $version);
    }

    /**
     * Publish after the core's check
     * @return array<string, mixed>
     */
    public function publish(EntityKey $key, ?int $version): array
    {
        $row = $this->store->read((string) $key, true, $version ?? 0);
        $doc = json_decode((string) $row['json']);
        if (!$doc instanceof stdClass) throw new ApiError(422, 'invalid_content', 'Not a scene document.', ['problems' => ['/: not an object']]);
        $problems = $this->core->sceneProblems((string) $key->id, $doc);
        if ($problems !== []) throw new ApiError(422, 'invalid_scene', 'This scene has problems a deck would not play.', ['problems' => $problems]);
        return $this->store->publish((string) $key, $version);
    }
}
