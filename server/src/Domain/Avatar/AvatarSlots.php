<?php
declare(strict_types=1);

namespace Scutoons\Domain\Avatar;

/**
 * The slot types of the avatar model (owner 2026-10-04: "slots are merely enums for an avatar model") – code, not
 * catalog data. Served as `slots` in GET /api/items/ (the place they had in config/items.json). Mirror of
 * @scutoons/content `AVATAR_SLOTS` (items.ts); CatalogTest keeps both lists equal.
 */
final class AvatarSlots
{
    public const ALL = ['head', 'face', 'neck', 'torso', 'hands', 'legs', 'feet', 'back', 'accessory'];

    public static function isSlot(mixed $slot): bool
    {
        return is_string($slot) && in_array($slot, self::ALL, true);
    }
}
