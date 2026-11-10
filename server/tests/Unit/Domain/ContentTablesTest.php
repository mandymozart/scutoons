<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Generated\ContentTables;

/** The generated content tables (#477, server/schema/tables): the one locale order runs the cases every language runs */
final class ContentTablesTest extends TestCase
{
    public function testTheSharedLocaleOrderCases(): void
    {
        $file = json_decode((string) file_get_contents(__DIR__ . '/../../../schema/tables/locale-order.vectors.json'), true, 512, JSON_THROW_ON_ERROR);
        self::assertGreaterThan(5, count($file['cases']));
        foreach ($file['cases'] as $case) {
            self::assertSame($case['expect'], ContentTables::pickText($case['text'], $case['asked'], $case['source']), (string) json_encode($case));
            // a decoded JSON object reads the same
            self::assertSame($case['expect'], ContentTables::pickText((object) $case['text'], $case['asked'], $case['source']));
        }
        self::assertNull(ContentTables::pickText('Katze', 'de'), 'a plain string is no locale map');
        self::assertSame('Katze', ContentTables::pickText(['de' => 'Katze', 'x' => 3], 'x'), 'only strings count');
    }

    public function testLegacyCharacterIdsKeepThePersona(): void
    {
        self::assertSame(['character' => 'catpope', 'persona' => 'human'], ContentTables::LEGACY_CHARACTER_IDS['plf']);
        self::assertSame(['plf', 'cat', 'mountain-tiger'], array_keys(ContentTables::LEGACY_CHARACTER_IDS));
    }
}
