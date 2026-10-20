<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Http\Middleware\JsonBody;

/**
 * A raw `PUT` body (a file upload: the asset manager, the storyboard's work storage) streamed into a temp file – never a
 * whole video in memory (JsonBody leaves bodies over 1 MB a stream). The caller deletes the file.
 */
final class RawUpload
{
    /** The temp file's path; 413 file_too_large above $max bytes, 422 empty_file without a body */
    public static function toTempFile(Request $request, int $max, string $prefix = 'upload-'): string
    {
        $declared = (int) $request->getHeaderLine('Content-Length');
        if ($declared > $max) throw new ApiError(413, 'file_too_large', "The file is larger than $max bytes.");
        $tmp = (string) tempnam(sys_get_temp_dir(), $prefix);
        $out = fopen($tmp, 'wb');
        if ($out === false) throw new ApiError(500, 'upload_failed', 'No temp file.');
        $raw = $request->getAttribute(JsonBody::ATTRIBUTE);
        $written = 0;
        if (is_string($raw)) {
            $written = (int) fwrite($out, $raw);
        } else {
            $body = $request->getBody();
            if ($body->isSeekable()) $body->rewind();
            while (!$body->eof()) {
                $chunk = $body->read(1048576);
                $written += (int) fwrite($out, $chunk);
                if ($written > $max) break;
            }
        }
        fclose($out);
        if ($written > $max) {
            @unlink($tmp);
            throw new ApiError(413, 'file_too_large', "The file is larger than $max bytes.");
        }
        if ($written === 0) {
            @unlink($tmp);
            throw new ApiError(422, 'empty_file', 'The request carried no file.');
        }
        return $tmp;
    }
}
