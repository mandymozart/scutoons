<?php
declare(strict_types=1);

namespace Scutoons\Domain\Live;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Catalog\PublishOrder;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Library\SceneContent;
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
 * kind `scenes`, as for shows and rigs): publishing asks it with the rigs its holders play (scene-content's answer, as a deck
 * gets it, #815), a scene a deck could not load is refused with the core's words.
 * Without the service nothing is published, 503. A draft never leaves the editor: only the published version is public.
 */
final class Scenes
{
    public function __construct(
        private readonly EntityStore $store,
        private readonly CoreValidator $core,
        private readonly CatalogValidator $catalog,
        private readonly SceneContent $content,
    ) {
    }

    /**
     * #815: scene-content's answer for the rigs the scene's holders play – what a deck asks for it (deck-space
     * published.rs `content_path`: each filler's own rig and each pinned one, PublishOrder::refs' `rig:` subjects, asked
     * as `character.<id>[.<persona>]` → `<id>[-<persona>]`, `item.<id>` → `<id>`); which rig plays is the core's
     * (`SceneRigs::played`). Null when it names none, as a deck then asks nothing.
     */
    public function rigs(EntityKey $key, stdClass $doc): ?stdClass
    {
        $asked = ['character' => [], 'item' => []];
        foreach (PublishOrder::refs($key, json_decode((string) json_encode($doc), true)) as $ref) {
            if (!str_starts_with($ref, 'rig:')) continue;
            $subject = explode('.', substr($ref, strlen('rig:')), 2);
            if (count($subject) !== 2 || !isset($asked[$subject[0]])) continue;
            $id = $subject[0] === 'character' ? preg_replace('/\./', '-', $subject[1], 1) : $subject[1];
            if (is_string($id) && preg_match(SceneContent::ID, $id) === 1 && !in_array($id, $asked[$subject[0]], true) && count($asked[$subject[0]]) < SceneContent::MAX_IDS) $asked[$subject[0]][] = $id;
        }
        if ($asked['character'] === [] && $asked['item'] === []) return null;
        $answer = json_decode(SceneContent::json($this->content->of([], $asked['character'], $asked['item'])));
        return $answer instanceof stdClass ? $answer : null;
    }

    /**
     * The catalog's descriptors of the `module:<id>` kinds the scene's nodes name and the instruments of its holders' rigs
     * (`$rigs`, scene-content's answer: a placed drum machine's voices), what the core registers before it builds the
     * graph (a module the catalog lacks is left out: the core names it)
     * @return list<stdClass>
     */
    public function modules(?stdClass $doc, ?stdClass $rigs = null): array
    {
        $nodes = $doc !== null && is_array($doc->node ?? null) ? $doc->node : [];
        $ids = [];
        foreach ($nodes as $node) {
            $kind = $node instanceof stdClass ? ($node->kind ?? null) : null;
            if (is_string($kind) && str_starts_with($kind, 'module:')) $ids[] = substr($kind, strlen('module:'));
        }
        $instruments = array_map(fn(mixed $e) => $e instanceof stdClass ? ($e->data ?? null) : null, array_values((array) ($rigs->instruments ?? [])));
        return $this->catalog->moduleDescriptors(array_values(array_unique([...$ids, ...CatalogValidator::moduleKinds($instruments)])));
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
        $rigs = $this->rigs($key, $doc);
        $problems = $this->core->sceneProblems((string) $key->id, $doc, $this->modules($doc, $rigs), $rigs);
        if ($problems !== []) throw new ApiError(422, 'invalid_scene', 'This scene has problems a deck would not play.', ['problems' => $problems]);
        return $this->store->publish((string) $key, $version);
    }
}
