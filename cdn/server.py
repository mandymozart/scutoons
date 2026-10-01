# Local CDN for development: serves cdn/root/ with CORS on http://localhost:<port> (default: "cdn" in
# shared/dev-ports.json, the single source of the local dev ports).
#
# Threaded (one thread per connection) and dual-stack: the old single-threaded HTTPServer hung whenever a browser
# opened a connection without sending a request right away (Chrome preconnects / idle keep-alive sockets): the one
# thread blocked on that socket and every other image request queued until the browser gave up
# (agents/PERFORMANCE.md). It also listened on 127.0.0.1 only, while the apps' `localhost` resolves to ::1 first.
#
# Answers single HTTP Range requests (`206 Partial Content`, `416` when unsatisfiable) so videos can seek (the
# music-video player of the prologue); production hosting serves ranges natively.
import email.utils
import json
import os
import re
import shutil
import socket
import sys
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer


RANGE = re.compile(r'^bytes=(\d*)-(\d*)$')


class CORSRequestHandler(SimpleHTTPRequestHandler):
    # idle sockets are closed after this many seconds instead of holding a thread forever
    timeout = 30
    # bytes left to send of a 206 answer (None = the whole file)
    _range_left = None

    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Range')
        self.send_header('Access-Control-Expose-Headers', 'Content-Length, Content-Range, Accept-Ranges')
        self.send_header('Accept-Ranges', 'bytes')
        SimpleHTTPRequestHandler.end_headers(self)

    def send_head(self):
        self._range_left = None
        header = self.headers.get('Range')
        path = self.translate_path(self.path)
        if not header or os.path.isdir(path) or not os.path.isfile(path):
            return super().send_head()
        match = RANGE.match(header.strip())
        if not match or (not match.group(1) and not match.group(2)):
            return super().send_head()  # multiple / malformed ranges: the whole file
        size = os.path.getsize(path)
        if match.group(1):
            start = int(match.group(1))
            end = min(int(match.group(2)), size - 1) if match.group(2) else size - 1
        else:  # suffix range: the last N bytes
            start = max(0, size - int(match.group(2)))
            end = size - 1
        if start >= size or start > end:
            self.send_response(416)
            self.send_header('Content-Range', f'bytes */{size}')
            self.send_header('Content-Length', '0')
            self.end_headers()
            return None
        f = open(path, 'rb')
        f.seek(start)
        self._range_left = end - start + 1
        self.send_response(206)
        self.send_header('Content-Type', self.guess_type(path))
        self.send_header('Content-Range', f'bytes {start}-{end}/{size}')
        self.send_header('Content-Length', str(self._range_left))
        self.send_header('Last-Modified', email.utils.formatdate(os.path.getmtime(path), usegmt=True))
        self.end_headers()
        return f

    def copyfile(self, source, outputfile):
        left = self._range_left
        if left is None:
            return shutil.copyfileobj(source, outputfile)
        while left > 0:
            chunk = source.read(min(64 * 1024, left))
            if not chunk:
                break
            outputfile.write(chunk)
            left -= len(chunk)

    def do_OPTIONS(self):
        self.send_response(200)
        self.end_headers()

    def log_message(self, format, *args):
        # errors only (4xx/5xx); a chapter page loads 100+ files
        if len(args) >= 2 and str(args[1])[:1] in ('4', '5'):
            super().log_message(format, *args)


class Server(ThreadingHTTPServer):
    daemon_threads = True
    request_queue_size = 64


class Server6(Server):
    address_family = socket.AF_INET6


if __name__ == '__main__':
    if len(sys.argv) > 1:
        port = int(sys.argv[1])
    else:
        with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'shared', 'dev-ports.json'), encoding='utf-8') as f:
            port = int(json.load(f)['cdn'])

    # serve cdn/root/ (relative to this file, so the server can be started from any folder)
    os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'root'))

    # loopback only, on both localhost addresses (::1 and 127.0.0.1)
    servers = [Server(('127.0.0.1', port), CORSRequestHandler)]
    try:
        servers.append(Server6(('::1', port), CORSRequestHandler))
    except OSError:
        pass  # no IPv6 loopback
    for extra in servers[1:]:
        threading.Thread(target=extra.serve_forever, daemon=True).start()
    print(f"Server running on http://localhost:{port}")
    print(f"Serving files from: {os.getcwd()}")
    try:
        servers[0].serve_forever()
    except KeyboardInterrupt:
        pass
