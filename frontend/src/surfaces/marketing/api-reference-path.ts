export function apiReferencePath(method: string, path: string) {
  // Encode path segments independently. Preserve separators so literal and parameter names stay distinct.
  return "/docs/api/" + method.toLowerCase() + path.split("/").map(encodeURIComponent).join("/");
}
