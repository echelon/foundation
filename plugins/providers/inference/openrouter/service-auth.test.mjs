import assert from 'node:assert/strict';
import test from 'node:test';
import { serviceAuthorization } from './service-auth.mjs';

const current = 'new-fixture-service-token-0123456789';
const previous = 'prior-fixture-service-token-0123456789';
const expires = '2030-01-02T03:04:05Z';
const header = (key) => `Bearer ${key}`;
const request = (...values) => ({ headersDistinct: { authorization: values } });

test('accepts both service tokens until the deadline and then retires the previous token', (t) => {
  const deadline = Date.parse(expires);
  let now = deadline - 1;
  let elapsed = 0;
  t.mock.method(Date, 'now', () => now);
  t.mock.method(performance, 'now', () => elapsed);
  const authorize = serviceAuthorization(current, { previousToken: previous, previousTokenExpiresAt: expires });
  assert.equal(authorize(request(header(current))), true);
  assert.equal(authorize(request(header(previous))), true);
  assert.equal(authorize(request(header('unrelated'))), false);
  assert.equal(authorize(request(header(current), header(previous))), false);
  assert.equal(authorize(request()), false);
  now = deadline;
  assert.equal(authorize(request(header(previous))), false);
  assert.equal(authorize(request(header(current))), true);
  now -= 100;
  assert.equal(authorize(request(header(previous))), false);
  const restarted = serviceAuthorization(current, { previousToken: previous, previousTokenExpiresAt: expires });
  now -= 100;
  elapsed += 100;
  assert.equal(restarted(request(header(previous))), false);
});

test('requires a valid, distinct previous token paired with a UTC expiry', () => {
  assert.throws(() => serviceAuthorization(current, { previousToken: previous }), /UTC timestamp/);
  assert.throws(() => serviceAuthorization(current, { previousTokenExpiresAt: expires }), /distinct previous service token/);
  assert.throws(() => serviceAuthorization(current, { previousToken: current, previousTokenExpiresAt: expires }), /distinct/);
  assert.throws(() => serviceAuthorization(current, { previousToken: 'short', previousTokenExpiresAt: expires,
    minimumPreviousLength: 24 }), /distinct/);
  assert.throws(() => serviceAuthorization(current, { previousToken: previous,
    previousTokenExpiresAt: '2030-02-30T03:04:05Z' }), /UTC timestamp/);
  const expired = serviceAuthorization(current, { previousToken: previous, previousTokenExpiresAt: '2000-01-01T00:00:00Z' });
  assert.equal(expired(request(header(previous))), false);
  assert.equal(expired(request(header(current))), true);
});
