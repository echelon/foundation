export class TokenCraftError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
    readonly requestId?: string,
    options?: ErrorOptions,
  ) {
    super(message, options);
    this.name = "TokenCraftError";
  }
}

export class TokenCraftTimeoutError extends Error {
  constructor(readonly timeoutMs: number, readonly requestId?: string) {
    super(`TokenCraft operation exceeded ${timeoutMs} ms.`);
    this.name = "TokenCraftTimeoutError";
  }
}

export function milliseconds(value: number, name: string): number {
  if (!Number.isInteger(value) || value <= 0 || value > 2_147_483_647) {
    throw new RangeError(`${name} must be a positive integer in milliseconds.`);
  }
  return value;
}

export function deadline(parent: AbortSignal | undefined, timeoutMs: number, requestId: () => string | undefined) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(new TokenCraftTimeoutError(timeoutMs, requestId())), timeoutMs);
  const relay = () => controller.abort(parent?.reason);
  if (parent?.aborted) relay();
  else parent?.addEventListener("abort", relay, { once: true });

  return {
    signal: controller.signal,
    close() {
      clearTimeout(timer);
      parent?.removeEventListener("abort", relay);
    },
  };
}

export function pause(duration: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    signal.throwIfAborted();
    const abort = () => {
      clearTimeout(timer);
      reject(signal.reason);
    };
    const timer = setTimeout(() => {
      signal.removeEventListener("abort", abort);
      resolve();
    }, duration);
    signal.addEventListener("abort", abort, { once: true });
  });
}

export function object(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

export function isRequestId(value: unknown): value is string {
  return typeof value === "string" && /^req_[0-9a-hjkmnp-tv-z]{26}$/.test(value);
}
