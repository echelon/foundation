"""Measure a bounded batch against the bundled TokenCraft connector."""

import argparse
from concurrent.futures import ThreadPoolExecutor
from http.client import HTTPConnection
import json
from math import ceil
from secrets import token_hex
from threading import Event
from time import monotonic, sleep
from typing import NamedTuple
from urllib.parse import urlencode, urlsplit


CLIENTS = ("load_client_a", "load_client_b")
IMAGE = {
    "model": "example_image",
    "task": "text_to_image",
    "prompt": "Synthetic connector load check",
    "count": 1,
    "aspect_ratio": "4:3",
}

class RequestSample(NamedTuple):
    identifier: str
    client: str
    admission_seconds: float
    terminal_seconds: float
    polls: int
    unchanged: int


def call(address, path, *, body=None, headers=None, expected=(200,)):
    connection = HTTPConnection(*address, timeout=8)
    headers = dict(headers or {})
    if body is not None:
        headers["Content-Type"] = "application/json"
        body = json.dumps(body).encode()
    try:
        connection.request("GET" if body is None else "POST", path, body, headers)
        response = connection.getresponse()
        if response.status not in expected:
            raise RuntimeError(f"{path.split('?')[0]} returned HTTP {response.status}")
        raw = response.read()
        return json.loads(raw) if raw else None, response.headers
    finally:
        connection.close()


def provision(address):
    _, headers = call(address, "/v1/auth/signup", expected=(201,), body={
        "email": f"load-{token_hex(8)}@example.test",
        "password": token_hex(24),
        "display_name": "Disposable load check",
    })
    session = {"Cookie": headers["Set-Cookie"].split(";", 1)[0]}
    accounts, _ = call(address, "/v1/accounts", headers=session)
    account = accounts["accounts"][0]["account_id"]
    query = urlencode({"account_id": account})
    project, _ = call(address, f"/v1/projects?{query}", headers=session,
                      expected=(201,), body={"name": "Connector load check"})
    project = project["id"]
    key, _ = call(address, f"/v1/projects/{project}/service-principals?{query}",
                  headers=session, expected=(201,), body={
                      "name": "Load check",
                      "scopes": ["requests:submit", "requests:read", "usage:read"],
                  })
    scope = urlencode({"account_id": account, "project_id": project})
    return {"Authorization": f"Bearer {key['token']}"}, session, scope


def queues(address, session, scope):
    snapshot, _ = call(address, f"/v1/operations/queues?{scope}", headers=session)
    rows = snapshot["queues"]
    return {kind: sum(row["count"] for row in rows if row["kind"] == kind)
            for kind in ("queued", "running", "awaiting_reconciliation")}


def observe(address, session, scope, done):
    peaks = {"queued": 0, "running": 0, "awaiting_reconciliation": 0}
    samples = 0
    while not done.is_set():
        for kind, value in queues(address, session, scope).items():
            peaks[kind] = max(peaks[kind], value)
        samples += 1
        done.wait(0.25)
    return {"samples": samples, "peak": peaks}


def execute(address, bearer, client, deadline):
    if monotonic() >= deadline:
        raise RuntimeError("load check reached its deadline before admission")
    start = monotonic()
    receipt, _ = call(address, "/v1/image", body=IMAGE, expected=(202,), headers={
        **bearer,
        "Idempotency-Key": token_hex(16),
        "X-TokenCraft-Client-Ref": client,
    })
    admitted = monotonic()
    identifier = receipt["id"]
    path = receipt["status_url"]
    if path not in (f"/v1/model-requests/{identifier}", f"/v1/requests/{identifier}"):
        raise RuntimeError("unexpected request status URL")
    headers = dict(bearer)
    polls = 0
    unchanged = 0
    while monotonic() < deadline:
        value, reply = call(address, path, headers=headers, expected=(200, 304))
        polls += 1
        if value is None:
            unchanged += 1
        elif value["status"] == "succeeded":
            return RequestSample(identifier, client, admitted - start, monotonic() - start, polls, unchanged)
        elif value["status"] not in {"queued", "running"}:
            raise RuntimeError(f"image request reached unexpected status {value['status']}")
        if reply.get("ETag"):
            headers["If-None-Match"] = reply["ETag"]
        sleep(min(2, max(0.1, float(reply.get("Retry-After", "1")))))
    raise RuntimeError("load check timed out while polling")


def verify_client(address, bearer, client, expected):
    query = urlencode({"client_ref": client})
    received = set()
    after = None
    while True:
        cursor = urlencode({"after": after}) if after else ""
        path = f"/v1/requests?{query}" + (f"&{cursor}" if cursor else "")
        page, _ = call(address, path, headers=bearer)
        for row in page["requests"]:
            if row["id"] in received or row["client_ref"] != client:
                raise RuntimeError(f"invalid history page for {client}")
            received.add(row["id"])
        next_after = page["next_after"]
        if not next_after:
            break
        if next_after == after or not page["requests"] or len(received) > len(expected):
            raise RuntimeError(f"history cursor made no progress for {client}")
        after = next_after
    if received != expected:
        raise RuntimeError(f"history did not match admitted requests for {client}")
    usage, _ = call(address, f"/v1/usage?{query}", headers=bearer)
    totals = usage["totals"]
    if any(totals[field] != len(expected) for field in ("requests", "succeeded", "completed_outputs")):
        raise RuntimeError(f"usage did not match terminal requests for {client}")


def latency(values):
    values = sorted(values)
    return {name: round(1_000 * values[ceil(len(values) * rank) - 1], 1)
            for name, rank in (("p50", 0.5), ("p95", 0.95), ("max", 1))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api", default="http://localhost:4000")
    parser.add_argument("--requests", type=int, default=64)
    parser.add_argument("--concurrency", type=int, default=12)
    options = parser.parse_args()
    url = urlsplit(options.api)
    if (url.scheme != "http" or url.hostname not in {"localhost", "127.0.0.1"}
            or url.username is not None or url.path not in {"", "/"} or url.query or url.fragment):
        parser.error("--api must be a loopback HTTP origin")
    if not 1 <= options.requests <= 256 or not 1 <= options.concurrency <= 32:
        parser.error("use 1–256 requests and 1–32 concurrent clients")
    address = ("127.0.0.1", url.port or 80)

    ready, _ = call(address, "/ready")
    catalog, _ = call(address, "/v1/catalog/offers")
    offers = [(row["id"], row["provider"], row["adapter"]) for row in catalog["offers"]]
    if not ready.get("ready") or offers != [("example_image", "self_hosted", "connector_queue")]:
        parser.error("start the bundled connector with tokencraft dev")

    bearer, session, scope = provision(address)
    done = Event()
    start = monotonic()
    deadline = start + 180
    with ThreadPoolExecutor(max_workers=options.concurrency + 1) as pool:
        watcher = pool.submit(observe, address, session, scope, done)
        try:
            calls = (CLIENTS[index % len(CLIENTS)] for index in range(options.requests))
            records = list(pool.map(lambda client: execute(address, bearer, client, deadline), calls))
        finally:
            done.set()
        queue = watcher.result()
    elapsed = monotonic() - start
    if len({row.identifier for row in records}) != len(records):
        raise RuntimeError("independent submissions returned duplicate request IDs")
    client_counts = {}
    for client in CLIENTS:
        expected = {row.identifier for row in records if row.client == client}
        verify_client(address, bearer, client, expected)
        client_counts[client] = len(expected)
    if any(queues(address, session, scope).values()):
        raise RuntimeError("request queue remained active after completion")
    print(json.dumps({
        "requests": len(records),
        "concurrency": options.concurrency,
        "catalog_revision": catalog["revision"],
        "elapsed_seconds": round(elapsed, 2),
        "completed_per_second": round(len(records) / elapsed, 2),
        "admission_ms": latency([row.admission_seconds for row in records]),
        "terminal_observed_ms": latency([row.terminal_seconds for row in records]),
        "polls": sum(row.polls for row in records),
        "polls_not_modified": sum(row.unchanged for row in records),
        "queue": queue,
        "verified_client_totals": client_counts,
    }, indent=2))


if __name__ == "__main__":
    main()
