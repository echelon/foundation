import assert from 'node:assert/strict';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { queueServer, readJson, send } from './queue.mjs';
import { kinovi } from './provider.mjs';

const model = 'seedance-20';
const authorization = 'Bearer test-vendor-key';
const jobInput = { kind: 'video', prompt: 'a square', duration_seconds: 5, resolution: '720p', aspect_ratio: '16:9' };
const submitPath = '/api/v1/jobs/createTask';
const statusPath = '/api/v1/jobs/recordInfo?taskId=vendor-task';
const artifactType = 'video/mp4';
const acknowledgement = { taskId: 'vendor-task' };
const resultReply = (url) => ({ taskId: 'vendor-task', model, status: 'success', output: [{ url }] });
const check = (body) => {
  assert.equal(body.model, model);
  assert.equal(body.inputs.duration, '5');
  assert.equal(body.inputs.outputResolution, '720p');
  assert.equal(body.autoFix, false);
};

async function listen(server) {
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  return `http://127.0.0.1:${server.address().port}`;
}

async function close(server) {
  if (!server?.listening) return;
  server.closeAllConnections();
  await new Promise((resolve) => server.close(resolve));
}

test('maps queued work, persists the handle on restart, and fetches artifact bytes', async () => {
  const state = await mkdtemp(join(tmpdir(), 'tokencraft-plugin-'));
  let plugin;
  let submitCount = 0;
  let vendorOrigin;
  let creditLiteral = '90071992547409.917';
  let vendorStatus = 'success';
  const vendor = createServer(async (request, response) => {
    if (request.url !== '/artifact') assert.equal(request.headers.authorization, authorization);
    if (request.url === submitPath && request.method === 'POST') {
      submitCount += 1;
      check(await readJson(request));
      return send(response, 200, acknowledgement);
    }
    if (request.url === statusPath) {
      const reply = { ...resultReply(`${vendorOrigin}/artifact`), status: vendorStatus };
      response.writeHead(200, { 'content-type': 'application/json' });
      return response.end(`${JSON.stringify(reply).slice(0, -1)},"creditsUsed":${creditLiteral}}`);
    }
    if (request.url === '/artifact') {
      assert.equal(request.headers.authorization, undefined);
      response.writeHead(200, { 'content-type': artifactType });
      return response.end('fixture artifact');
    }
    return send(response, 404, {});
  });
  try {
    vendorOrigin = await listen(vendor);
    const provider = kinovi({ origin: vendorOrigin, key: 'test-vendor-key', artifactOrigins: [vendorOrigin] });
    const start = async () => {
      plugin = await queueServer({ state, token: 'test-gateway-key', provider });
      return listen(plugin);
    };
    let base = await start();
    const headers = { authorization: 'Bearer test-gateway-key', 'idempotency-key': 'example-1' };
    const job = { version: 1, request_id: 'example-1', model, input: jobInput };
    const submit = (body = job) => fetch(`${base}/v1/jobs`, { method: 'POST', headers, body: JSON.stringify(body) });
    const accepted = { version: 1, request_id: 'example-1', outcome: { status: 'accepted', handle: 'example-1' } };
    assert.deepEqual(await (await submit()).json(), accepted);
    assert.deepEqual(await (await submit()).json(), accepted);
    assert.equal((await (await submit({ ...job, input: { ...jobInput, prompt: 'changed' } })).json()).outcome.status,
      'uncertain');
    assert.equal(submitCount, 1);
    await close(plugin);
    base = await start();
    assert.deepEqual(await (await submit()).json(), accepted);
    assert.equal(submitCount, 1);
    assert.deepEqual(await (await fetch(`${base}/v1/jobs/example-1`, { headers })).json(),
      { version: 1, handle: 'example-1', status: 'succeeded' });
    const usage = async () => (await fetch(`${base}/v1/jobs/example-1`, {
      headers: { ...headers, 'x-tokencraft-provider-charge': '1' },
    })).json();
    assert.deepEqual((await usage()).provider_charge, { unit: 'credit', quantity: '90071992547409.917' });
    creditLiteral = '2.17e-3';
    assert.deepEqual((await usage()).provider_charge, { unit: 'credit', quantity: '0.00217' });
    for (const value of ['-2', '1e-10', '"2.17"', '1e+200', 'null']) {
      creditLiteral = value;
      assert.deepEqual(await usage(), { version: 1, handle: 'example-1', status: 'succeeded' });
    }
    creditLiteral = '2.17';
    for (const [upstreamStatus, expected] of [['waiting', 'queued'], ['generating', 'running'], ['fail', 'failed']]) {
      vendorStatus = upstreamStatus;
      assert.deepEqual(await usage(), { version: 1, handle: 'example-1', status: expected });
    }
    vendorStatus = 'success';
    assert.equal((await fetch(`${base}/v1/jobs/example-1/artifacts/0`)).status, 401);
    const artifact = await fetch(`${base}/v1/jobs/example-1/artifacts/0`, { headers });
    assert.equal(artifact.headers.get('content-type'), artifactType);
    assert.equal(await artifact.text(), 'fixture artifact');
  } finally {
    await close(plugin);
    await close(vendor);
    await rm(state, { recursive: true });
  }
});
