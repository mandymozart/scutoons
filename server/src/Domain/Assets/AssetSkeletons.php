<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Rigs\CoreValidator;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * A model's or an animation's skeleton (#817, migration 2026-11-23-asset-skeletons): what the deck's core read from the
 * .glb – joints, the joints its first clip turns, its clips, its family (deck-catalog skeleton.rs `Skeleton`). The API
 * keeps no rule of it: the core checks it before it is stored (`CoreValidator::skeletonProblems`, `/validate` kind
 * `skeleton`) on the asset's current version; `GET /api/assets/{id}` carries it per version, the list per asset.
 */
final class AssetSkeletons
{
    private ?bool $available = null;

    public function __construct(private readonly Database $db, private readonly AssetRegistry $registry, private readonly CoreValidator $core)
    {
    }

    /** Until the migration runs, versions have no skeletons */
    public function available(): bool
    {
        if ($this->available === null) {
            try {
                $this->db->value('SELECT COUNT(skeleton) FROM asset_versions WHERE 1 = 0');
                $this->available = true;
            } catch (Throwable) {
                $this->available = false;
            }
        }
        return $this->available;
    }

    /** The `skeleton` of a PUT /assets/{id} body: null = none, else the core's document, checked by the core */
    public function save(string $assetId, mixed $skeleton): void
    {
        if (!$this->available()) throw new ApiError(503, 'not_available', 'Skeletons are not set up yet (migration 2026-11-23-asset-skeletons).');
        $asset = $this->registry->asset($assetId) ?? throw new ApiError(404, 'not_found', 'Unknown asset.');
        if (!in_array($asset['kind'], ['model', 'animation'], true)) throw new ApiError(422, 'invalid_skeleton', 'Only a model or an animation has a skeleton.');
        $version = $this->registry->currentVersion($assetId) ?? throw new ApiError(404, 'not_found', 'This asset has no current version.');
        $json = null;
        if ($skeleton !== null) {
            if (!is_array($skeleton) || ($skeleton !== [] && array_is_list($skeleton))) throw new ApiError(422, 'invalid_skeleton', 'skeleton must be an object or null.');
            $json = (string) json_encode((object) $skeleton, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
            if (strlen($json) > 262144) throw new ApiError(422, 'invalid_skeleton', 'skeleton is too long.');
            $problems = $this->core->skeletonProblems($assetId, (object) $skeleton);
            if ($problems !== []) throw new ApiError(422, 'invalid_skeleton', 'This skeleton has problems.', ['problems' => $problems]);
        }
        $this->db->run('UPDATE asset_versions SET skeleton = ? WHERE id = ?', [$json, $version['id']]);
        $this->registry->touch($assetId);
    }

    /**
     * The stored skeleton of the file at $path (the current version of the asset it belongs to), null when none is stored
     * @return array<string, mixed>|null
     */
    public function ofPath(string $path): ?array
    {
        if (!$this->available()) return null;
        $row = $this->db->one("SELECT v.skeleton FROM asset_versions v JOIN assets a ON a.id = v.asset_id
            WHERE v.path = ? AND v.status = 'current' AND a.status = 'active' LIMIT 1", [$path]);
        return $row !== null ? self::of($row) : null;
    }

    /**
     * @param array<string, mixed> $version an asset_versions row (or a row with its `skeleton` column)
     * @return array<string, mixed>|null
     */
    public static function of(array $version): ?array
    {
        $skeleton = json_decode((string) ($version['skeleton'] ?? ''), true);
        return is_array($skeleton) ? $skeleton : null;
    }
}
