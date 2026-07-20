/**
 * Open-redirect guard for `?redirect_to=` params. Only same-origin path redirects survive:
 * the value must be an absolute path (`/…`) and must NOT be protocol-relative (`//evil.com`),
 * a backslash trick (`/\evil.com` — browsers normalize `\` to `/`), or carry a scheme
 * (`https:…`, `javascript:…`). As a final check the value is resolved by the real URL parser
 * against our origin and rejected if it lands anywhere else.
 */
export function safeRedirectPath(raw: string | null | undefined): string | null {
  if (!raw) return null;
  if (!raw.startsWith("/") || raw.startsWith("//") || raw.startsWith("/\\")) return null;
  try {
    const url = new URL(raw, window.location.origin);
    if (url.origin !== window.location.origin) return null;
    // `/..//evil.com` normalizes to a PATHNAME of `//evil.com` — same-origin by parse, but
    // protocol-relative if ever fed back into navigation. Reject any `//`-leading result.
    if (url.pathname.startsWith("//")) return null;
    return url.pathname + url.search + url.hash;
  } catch {
    return null;
  }
}
