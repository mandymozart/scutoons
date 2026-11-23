<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Rigs\CoreValidator;

/**
 * #817: a character is saved only with idles that play on its models. Each persona's `animations.idle` (a library
 * animation or a clip of its own model) is checked by the core against every model the persona can stand in (its own,
 * its looks') – deck-catalog skeleton.rs `FitCheck`, the build service's `/validate` kind `animation-fit`. The API keeps
 * no rule: it only gathers the stored skeletons (AssetSkeletons) and asks. A file whose skeleton is not read yet is
 * named – the creator reads and stores it when it opens the file.
 */
final class IdleCheck
{
    public function __construct(private readonly AssetSkeletons $skeletons, private readonly CoreValidator $core)
    {
    }

    /** Throws 422 invalid_content { problems } when an idle of `character:<id>` doesn't play on its model */
    public function guard(EntityKey $key, mixed $content): void
    {
        if ($key->kind !== 'character' || !$content instanceof \stdClass) return;
        $problems = $this->problems((string) $key->id, $content);
        if ($problems) {
            throw new ApiError(422, 'invalid_content', "An idle of character $key->id doesn't play on its model: " . $problems[0]
                . (count($problems) > 1 ? ' (+' . (count($problems) - 1) . ' more)' : ''), ['problems' => $problems]);
        }
    }

    /** @return list<string> */
    public function problems(string $character, \stdClass $content): array
    {
        $problems = [];
        foreach (is_array($content->personas ?? null) ? $content->personas : [] as $persona) {
            if (!$persona instanceof \stdClass) continue;
            $idle = ($persona->animations ?? null) instanceof \stdClass ? ($persona->animations->idle ?? null) : null;
            if (!$idle instanceof \stdClass) continue;
            $who = "character:$character/" . (is_string($persona->id ?? null) ? $persona->id : '?');
            $check = new \stdClass();
            if (is_string($idle->animation ?? null)) {
                $animation = $this->skeleton($idle->animation, $who, 'animation', $problems);
                if ($animation === null) continue;
                $check->animation = $animation;
            }
            if (is_string($idle->clip ?? null)) $check->clip = $idle->clip;
            foreach (self::models($persona) as $model) {
                $skeleton = $this->skeleton($model, $who, 'model', $problems);
                if ($skeleton === null) continue;
                $check->model = $skeleton;
                array_push($problems, ...$this->core->fitProblems($who, $check));
            }
        }
        return $problems;
    }

    /**
     * the models a persona stands in: its own, its looks' (none = no model, nothing to play on)
     * @return list<string>
     */
    private static function models(\stdClass $persona): array
    {
        $models = is_string($persona->model ?? null) && trim($persona->model) !== '' ? [trim($persona->model)] : [];
        foreach (is_array($persona->looks ?? null) ? $persona->looks : [] as $look) {
            if ($look instanceof \stdClass && is_string($look->model ?? null) && trim($look->model) !== '') $models[] = trim($look->model);
        }
        return array_values(array_unique($models));
    }

    /** @param list<string> $problems */
    private function skeleton(string $path, string $who, string $what, array &$problems): ?object
    {
        $key = AssetPaths::normalize($path);
        $skeleton = $key !== null ? $this->skeletons->ofPath($key) : null;
        if ($skeleton === null) {
            $problems[] = "$who: the $what $path has no skeleton read yet – open it in the creator's library";
            return null;
        }
        return json_decode((string) json_encode((object) $skeleton));
    }
}
