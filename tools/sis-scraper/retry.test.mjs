import { test } from 'node:test';
import assert from 'node:assert/strict';
import { retry } from './retry.mjs';

test('returns the first success without retrying', async () => {
  let calls = 0;
  assert.equal(await retry(async () => ++calls, { waitMs: 0 }), 1);
  assert.equal(calls, 1);
});

test('retries until a later attempt succeeds', async () => {
  let calls = 0;
  const retried = [];
  const value = await retry(
    async () => {
      if (++calls < 3) throw new Error(`fail ${calls}`);
      return 'ok';
    },
    { waitMs: 0, onRetry: (e, attempt) => retried.push([e.message, attempt]) }
  );
  assert.equal(value, 'ok');
  assert.deepEqual(retried, [['fail 1', 1], ['fail 2', 2]]);
});

test('throws the last error once every attempt fails', async () => {
  let calls = 0;
  await assert.rejects(
    retry(async () => {
      throw new Error(`fail ${++calls}`);
    }, { tries: 2, waitMs: 0 }),
    /fail 2/
  );
  assert.equal(calls, 2);
});
