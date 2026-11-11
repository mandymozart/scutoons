<?php
declare(strict_types=1);

namespace Scutoons\Domain\Live;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Rigs\CoreValidator;
use stdClass;

/**
 * Show documents (#90, research/SHOW.md §4b; one per conductor, the seats saved in the parent's): `show:<id>` in entity_versions,
 * drafts and history like every document. The `version` a seat pins is the number the server stamps on each change of a
 * document, kept with it; `entity_versions.id` is the save, a pin never uses it. What makes a show valid is the Rust core's
 * (deck-show, over the build service's `POST /validate`, as for rigs, #313): drafts are saved unchecked, publishing asks it
 * with every show the tree names by value. Without the service nothing is published, 503.
 */
final class Shows
{
    public function __construct(private readonly EntityStore $store, private readonly CoreValidator $core)
    {
    }

    /**
     * A new draft: the document names itself, the version comes from here (EntityStore::saveStamped).
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function save(array $user, EntityKey $key, stdClass $body): array
    {
        return $this->store->saveStamped($user, $key, $body, 'show');
    }

    /** The document at an exact version a seat pins */
    public function atVersion(string $id, int $version): ?stdClass
    {
        return $this->store->atStamp("show:$id", $version);
    }

    /**
     * Publish after the core's check of the whole tree.
     * @return array<string, mixed>
     */
    public function publish(string $key, ?int $version): array
    {
        $row = $this->store->read($key, true, $version ?? 0);
        $doc = json_decode((string) $row['json']);
        if (!$doc instanceof stdClass) throw new ApiError(422, 'invalid_content', 'Not a show document.', ['problems' => ['/: not an object']]);
        $problems = $this->core->showProblems((string) ($doc->id ?? ''), $doc, $this->library($doc));
        if ($problems !== []) throw new ApiError(422, 'invalid_show', 'This show has problems.', ['problems' => $problems]);
        return $this->store->publish($key, $version);
    }

    /**
     * Every show the tree under $doc names, by value, each once. A seat that finds nothing is left out: the core reports it.
     * @return list<stdClass>
     */
    private function library(stdClass $doc): array
    {
        $seen = [];
        $found = [];
        $todo = [$doc];
        while ($todo !== []) {
            $show = array_pop($todo);
            foreach (is_array($show->seats ?? null) ? $show->seats : [] as $seat) {
                $pin = $seat->show ?? null;
                if (!$pin instanceof stdClass || !is_string($pin->id ?? null) || !is_int($pin->version ?? null)) continue;
                $name = $pin->id . '@' . $pin->version;
                if (isset($seen[$name])) continue;
                $seen[$name] = true;
                $child = $this->atVersion($pin->id, $pin->version);
                if ($child !== null) {
                    $found[] = $child;
                    $todo[] = $child;
                }
            }
        }
        return $found;
    }
}
