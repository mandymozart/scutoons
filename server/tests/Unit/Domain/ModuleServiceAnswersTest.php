<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;
use Scutoons\Domain\Modules\ModuleCompiler;

/**
 * #416: the build service's REAL answers for the three factory modules (walk, analog, fixture – captured from the service
 * with `CAPTURE_SERVICE_ANSWERS=1 cargo test -p scutoons-build --test real_compile`, deck/build/tests/real_compile.rs §7)
 * are modules to the API. A field the core's derive adds to the descriptor without ModuleSource knowing it made every
 * compile answer 502 "not a module" in production; deck/build's test checks the same lists from the other side. No database.
 */
final class ModuleServiceAnswersTest extends TestCase
{
    /** @return array<string, array{string}> */
    public static function factoryModules(): array
    {
        return ['walk' => ['walk'], 'analog' => ['analog'], 'fixture' => ['fixture']];
    }

    /** @return array<string, mixed> */
    private static function answer(string $id): array
    {
        $file = dirname(__DIR__, 2) . "/fixtures/modules/service-answer-$id.json";
        return (array) json_decode((string) file_get_contents($file), true, 512, JSON_THROW_ON_ERROR);
    }

    #[DataProvider('factoryModules')]
    public function testTheServicesRealAnswerIsAModule(string $id): void
    {
        $answer = self::answer($id);
        self::assertSame([], ModuleCompiler::answerProblems($answer), "the API refuses the service's answer for $id");
        self::assertSame($id, $answer['descriptor']['name']);
        self::assertSame($answer['hash'], hash('sha256', (string) base64_decode((string) $answer['wasm_b64'], true)), 'the hash is the file\'s');
    }

    public function testAProblemIsNamed(): void
    {
        $answer = self::answer('walk');
        $answer['descriptor']['abi'] = 1;
        $answer['descriptor']['params'][0]['knob'] = true;
        $problems = ModuleCompiler::answerProblems($answer);
        self::assertContains('descriptor.abi must be 2 (a module built against another ABI is recompiled)', $problems);
        self::assertContains('descriptor.params[0] has no field knob', $problems);
        self::assertSame(['the file is not a wasm'], ModuleCompiler::answerProblems(['wasm_b64' => base64_encode('GIF89a')] + self::answer('walk')));
        self::assertSame(['wasm_b64 is not base64 of a file', 'no descriptor'], ModuleCompiler::answerProblems([]));
    }
}
