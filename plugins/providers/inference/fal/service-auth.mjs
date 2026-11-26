import { createHash, timingSafeEqual } from 'node:crypto';

const digest = (value) => createHash('sha256').update(value).digest();
const bearer = (token) => digest(`Bearer ${token}`);
const tokenFormat = /^[A-Za-z0-9._~+/=-]{1,256}$/;

export function serviceAuthorization(token, { previousToken, previousTokenExpiresAt, minimumPreviousLength = 1 } = {}) {
  if (typeof token !== 'string' || !token) throw new Error('set the plugin service token');
  const current = bearer(token);
  let previous;
  let deadline;
  let elapsedDeadline;
  if (previousToken !== undefined || previousTokenExpiresAt !== undefined) {
    if (typeof previousToken !== 'string' || previousToken === token || !tokenFormat.test(previousToken)
        || previousToken.length < minimumPreviousLength) {
      throw new Error('configure a distinct previous service token');
    }
    deadline = Date.parse(previousTokenExpiresAt);
    const canonical = Number.isFinite(deadline) && new Date(deadline).toISOString();
    if (!canonical || (previousTokenExpiresAt !== canonical && previousTokenExpiresAt !== canonical.replace('.000Z', 'Z'))) {
      throw new Error('set the previous service token expiry to a UTC timestamp');
    }
    const remaining = deadline - Date.now();
    if (remaining > 0) {
      previous = bearer(previousToken);
      elapsedDeadline = performance.now() + remaining;
    }
  }

  return (request) => {
    const values = request.headersDistinct.authorization;
    if (values?.length !== 1) return false;
    // A correction to the system clock must never extend the overlap.
    if (previous && (Date.now() >= deadline || performance.now() >= elapsedDeadline)) previous = undefined;
    const supplied = digest(values[0]);
    const matchesCurrent = timingSafeEqual(current, supplied);
    const matchesPrevious = previous !== undefined && timingSafeEqual(previous, supplied);
    return matchesCurrent || matchesPrevious;
  };
}
