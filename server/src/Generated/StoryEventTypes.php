<?php
// GENERATED from server/schema/content/chapter.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
declare(strict_types=1);

namespace Scutoons\Generated;

/** the story events (#517 / #521): every VNEvent type the chapter schema takes, in its order */
final class StoryEventTypes
{
    /** @var list<string> */
    public const TYPES = ['showMessage', 'showChoices', 'addChoice', 'joinScene', 'leaveScene', 'hideMessage', 'showTextBox', 'fx', 'unlockLocation', 'migrate', 'setAvatarLocation', 'jumpTo', 'label', 'divert', 'setVariable', 'giveItem', 'takeItem', 'askText'];
}
