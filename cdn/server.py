# cors_server.py
from http.server import HTTPServer, SimpleHTTPRequestHandler
import sys
import os

class CORSRequestHandler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        SimpleHTTPRequestHandler.end_headers(self)

    def do_OPTIONS(self):
        self.send_response(200)
        self.end_headers()

if __name__ == '__main__':
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
    
    # Change to the root/ directory
    os.chdir('root')
    
    httpd = HTTPServer(('localhost', port), CORSRequestHandler)
    print(f"Server running on http://localhost:{port}")
    print(f"Serving files from: {os.getcwd()}")
    httpd.serve_forever()