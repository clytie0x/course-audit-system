// Retry one SIS page a few times. A single failure would otherwise turn a full
// run into a merge, and then courses SIS has withdrawn are never dropped.

/**
 * Run `fn` up to `tries` times, waiting `waitMs` between attempts. `onRetry`
 * gets the error and the attempt number that failed. The last error is thrown.
 */
export async function retry(fn, { tries = 3, waitMs = 5000, onRetry = () => {} } = {}) {
  for (let attempt = 1; ; attempt++) {
    try {
      return await fn();
    } catch (e) {
      if (attempt >= tries) throw e;
      onRetry(e, attempt);
      await new Promise((r) => setTimeout(r, waitMs));
    }
  }
}
