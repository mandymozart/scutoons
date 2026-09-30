# Local CDN for development: serves cdn/root/ with CORS on http://localhost:<port> (default 8001, the fixed dev port;
# see agents/PLATFORM.md "Local dev ports").
#
# Threaded (one thread per connection) and dual-stack: the old single-threaded HTTPServer hung whenever a browser
# opened a connection without sending a request right away (Chrome preconnects / idle keep-alive sockets): the one
# thread blocked on that socket and every other image request queued until the browser gave up
# (agents/PERFORMANCE.md). It also listened on 127.0.0.1 only, while the apps' `localhost` resolves to ::1 first.
import os
import socket
import sys
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer


class CORSRequestHandler(SimpleHTTPRequestHandler):
    # idle sockets are closed after this many seconds instead of holding a thread forever
    timeout = 30

    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        SimpleHTTPRequestHandler.end_headers(self)

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
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8001

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
