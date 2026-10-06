<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Domain\Content\NeutralDoc;

/**
 * Language-neutral documents (agents/NEUTRAL.md): the server's resolution gives exactly the answers of
 * @scutoons/content `resolveNeutral` – both read the golden fixtures shared/content/test/fixtures/neutral/*.json.
 */
final class NeutralDocTest extends TestCase
{
    private const FIXTURES = __DIR__ . '/../../../../shared/content/test/fixtures/neutral';
    private const JSON = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;

    public function testGoldenFixtures(): void
    {
        $files = glob(self::FIXTURES . '/*.json') ?: [];
        self::assertGreaterThanOrEqual(6, count($files));
        foreach ($files as $file) {
            $fixture = json_decode((string) file_get_contents($file));
            self::assertInstanceOf(\stdClass::class, $fixture, $file);
            $before = json_encode($fixture->doc, self::JSON);
            foreach ($fixture->cases as $case) {
                [$served, $fallback, $doc] = NeutralDoc::resolve($fixture->kind, $fixture->doc, $case->locale, $fixture->languages->source,
                    $fixture->languages->languages, $fixture->status ?? null);
                $label = basename($file) . " $case->locale";
                self::assertSame([$case->expected->locale, $case->expected->fallback], [$served, $fallback], $label);
                self::assertSame(json_encode($case->expected->doc, self::JSON), json_encode($doc, self::JSON), $label);
            }
            self::assertSame($before, json_encode($fixture->doc, self::JSON), basename($file) . ': the stored document is never changed');
        }
    }

    public function testBlankIsMissingLikeJavaScriptTrim(): void
    {
        foreach (['', ' ', "\t\n", "\u{00A0}", "\u{2003}\u{FEFF}", [], [''], [' ', "\u{00A0}"], null, 1] as $blank) {
            self::assertFalse(NeutralDoc::isText($blank), (string) json_encode($blank));
        }
        foreach (['a', ' a ', ['', 'x']] as $text) self::assertTrue(NeutralDoc::isText($text), (string) json_encode($text));
        self::assertSame('Hallo', NeutralDoc::text((object) ['de' => 'Hallo', 'en' => "\u{00A0}"], ['en', 'de']));
        self::assertSame('legacy', NeutralDoc::text('legacy', ['de']));
        self::assertNull(NeutralDoc::text(new \stdClass(), ['de']));
    }
}
