import assert from 'node:assert/strict';
import { test } from 'node:test';
import { delegatedKey, keyTag, matchesKey } from './tenant-keys.mjs';

test('delegation is explicit and the persisted tag is bound to one job and key', () => {
  const header = 'x-tokencraft-upstream-key';
  const key = 'fixture-fal-key:credential';
  const request = (values) => ({ headersDistinct: values ? { [header]: values } : {} });
  assert.equal(delegatedKey(request(), false), undefined);
  assert.equal(delegatedKey(request([key]), false), null);
  assert.equal(delegatedKey(request(), true), null);
  assert.equal(delegatedKey(request([key, key]), true), null);
  assert.equal(delegatedKey(request(['short']), true), null);
  assert.equal(delegatedKey(request([key]), true), key);
  const tag = keyTag(key, 'request-1');
  assert.match(tag, /^[a-f0-9]{64}$/);
  assert.ok(matchesKey(tag, keyTag(key, 'request-1')));
  assert.ok(!matchesKey(tag, keyTag(key, 'request-2')));
  assert.ok(!matchesKey(tag, keyTag(key + '-different', 'request-1')));
  assert.ok(!matchesKey(tag, undefined));
});
