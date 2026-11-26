import { createHmac, timingSafeEqual } from 'node:crypto';

const header = 'x-tokencraft-upstream-key';
const keyShape = /^[A-Za-z0-9_.:~+/=-]{16,128}$/;

export function configuredKeyMode(keyName, modeName, token) {
  const mode = process.env[modeName] ?? 'false';
  if (!['true', 'false'].includes(mode)) throw new Error(`set ${modeName} to true or false`);
  const tenantKeys = mode === 'true';
  const key = process.env[keyName];
  if (tenantKeys ? key || token.length < 32 : !key) {
    throw new Error(`configure ${keyName} for operator mode, or set ${modeName} with an independent service token`);
  }
  return { key, tenantKeys };
}

export function delegatedKey(request, tenantKeys) {
  const values = request.headersDistinct[header];
  if (!tenantKeys) return values === undefined ? undefined : null;
  return values?.length === 1 && keyShape.test(values[0]) ? values[0] : null;
}

export function keyTag(key, id) {
  return key && createHmac('sha256', key).update('tokencraft/queued-provider-key/v1/' + id).digest('hex');
}

export function matchesKey(saved, incoming) {
  if (saved === undefined || incoming === undefined) return saved === incoming;
  return /^[a-f0-9]{64}$/.test(saved) && timingSafeEqual(Buffer.from(saved), Buffer.from(incoming));
}
