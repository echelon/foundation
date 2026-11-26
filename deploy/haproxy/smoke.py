"""Exercise the cell ingress against disposable loopback API replicas."""

import argparse
from concurrent.futures import ThreadPoolExecutor
from contextlib import ExitStack, closing
from http.client import HTTPConnection, HTTPException
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import os
from pathlib import Path
import socket
import subprocess
from tempfile import TemporaryDirectory
from threading import Event
import time


PROBE = "/_tokencraft/ingress-ready"
FORWARD = {"X-Forwarded-For": "192.0.2.42"}
CONFIG = Path(__file__).with_name("cell-api.cfg")


class Replica(ThreadingHTTPServer):
    def __init__(self, name):
        super().__init__(("127.0.0.1", 0), Api)
        self.name = name
        self.ready = Event()
        self.finish_stream = Event()
        self.submissions = 0
        self.ready.set()


class Api(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        if self.path == "/ready":
            self.send_response(200 if self.server.ready.is_set() else 503)
            self.end_headers()
        elif self.path == "/stream":
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.wfile.write(b"data: first\n\n")
            self.wfile.flush()
            if self.server.finish_stream.wait(6):
                self.wfile.write(b"data: finish\n\n")
        else:
            self.send_response(200)
            self.end_headers()
            original = self.headers.get("X-Forwarded-For")
            scheme = self.headers.get("X-Forwarded-Proto")
            self.wfile.write(f"{self.server.name} {original} {scheme}".encode())

    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", "0")))
        self.server.submissions += 1
        self.connection.shutdown(socket.SHUT_RDWR)


def request(port, method, path, headers=None, body=None):
    with closing(HTTPConnection("127.0.0.1", port, timeout=2)) as client:
        client.request(method, path, headers=headers or {}, body=body)
        response = client.getresponse()
        return response.status, response.read()


def wait_for(port, status, process):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"HAProxy exited with status {process.returncode}")
        try:
            if request(port, "GET", PROBE)[0] == status:
                return
        except (OSError, HTTPException):
            pass
        time.sleep(0.1)
    raise TimeoutError(f"Ingress readiness did not become {status}")


def check_stream(port, replicas):
    with closing(HTTPConnection("127.0.0.1", port, timeout=2)) as client:
        client.request("GET", "/stream", headers=FORWARD)
        response = client.getresponse()
        assert response.status == 200, response.status
        assert response.readline() == b"data: first\n"
        for replica in replicas:
            replica.finish_stream.set()
        assert response.read() == b"\ndata: finish\n\n"


def check_routes(port, replicas, process):
    wait_for(port, 200, process)
    assert request(port, "GET", "/who")[0] == 400
    assert request(port, "GET", "/who", {"X-Forwarded-For": "bad"})[0] == 400
    assert request(port, "GET", "/who", {"X-Forwarded-For": "192.0.2.1, 192.0.2.2"})[0] == 400
    expected = {(200, f"{replica.name} 192.0.2.42 https".encode()) for replica in replicas}
    responses = set()
    deadline = time.monotonic() + 10
    while responses != expected and time.monotonic() < deadline:
        response = request(port, "GET", "/who", {**FORWARD, "X-Forwarded-Proto": "http"})
        assert response in expected, response
        responses.add(response)
        time.sleep(0.05)
    assert responses == expected, responses
    check_stream(port, replicas)

    # A replica that received a POST can lose the connection before replying.
    status, _ = request(port, "POST", "/submit", FORWARD, b"fixture")
    assert status >= 500, status
    assert sum(replica.submissions for replica in replicas) == 1

    for replica in replicas:
        replica.ready.clear()
    wait_for(port, 503, process)
    assert request(port, "GET", "/who", FORWARD)[0] == 503
    replicas[1].ready.set()
    wait_for(port, 200, process)
    for _ in range(4):
        assert request(port, "GET", "/who", FORWARD) == (200, b"second 192.0.2.42 https")


def main(haproxy):
    with TemporaryDirectory() as temporary, ExitStack() as cleanup:
        directory = Path(temporary)
        allowlist = directory / "trusted.txt"
        allowlist.write_text("127.0.0.1/32\n")
        with closing(socket.socket()) as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        environment = {
            **os.environ,
            "TOKENCRAFT_LB_BIND": f"127.0.0.1:{port}",
            "TOKENCRAFT_LB_TRUSTED_INGRESS_FILE": str(allowlist),
            "TOKENCRAFT_LB_MAX_CONNECTIONS": "64",
            "TOKENCRAFT_LB_CONNECTIONS_PER_API": "16",
            "TOKENCRAFT_LB_API_SLOTS": "2",
            "TOKENCRAFT_LB_API_ADDRESS": "api.cell.test:4000",
        }
        subprocess.run([haproxy, "-c", "-f", str(CONFIG)], env=environment, check=True)

        replicas = [Replica("first"), Replica("second")]
        threads = cleanup.enter_context(ThreadPoolExecutor(max_workers=2))
        for replica in replicas:
            cleanup.callback(replica.server_close)
            threads.submit(replica.serve_forever)
            cleanup.callback(replica.shutdown)

        # Use fixed loopback addresses for the traffic checks. The production
        # configuration above is parsed with its DNS discovery line intact.
        fixture = directory / "fixture.cfg"
        config = CONFIG.read_text().splitlines()
        servers = [line for line in config if line.strip().startswith("server-template ")]
        assert len(servers) == 1
        index = config.index(servers[0])
        config[index:index + 1] = [
            f"    server {replica.name} 127.0.0.1:{replica.server_port} check" for replica in replicas
        ]
        fixture.write_text("\n".join(config) + "\n")
        with (directory / "haproxy.log").open("w+") as log:
            process = subprocess.Popen(
                [haproxy, "-db", "-f", str(fixture)], env=environment, stdout=log, stderr=log,
            )
            try:
                check_routes(port, replicas, process)
            except Exception:
                log.seek(0)
                print(log.read())
                raise
            finally:
                if process.poll() is None:
                    process.terminate()
                try:
                    process.wait(5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        print("Cell ingress readiness, distribution, forwarding headers, SSE, and POST replay checks passed.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--haproxy", default="haproxy", help="HAProxy 3.2+ binary")
    main(parser.parse_args().haproxy)
