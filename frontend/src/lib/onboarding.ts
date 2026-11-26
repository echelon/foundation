export const connectPath = "/connect";

export function defaultSignupDestination(demo: boolean) {
  return demo ? connectPath : "/dashboard";
}

export function connectionCheckPath(demo: boolean) {
  return demo ? "/v1/requests" : "/v1/jobs";
}

export function isConnectDestination(destination: string) {
  return destination.split(/[?#]/, 1)[0] === connectPath;
}

export function entryPath(path: "/signup" | "/login", destination: string) {
  return `${path}?${new URLSearchParams({ redirect_to: destination })}`;
}

export function getStartedPath(signedIn: boolean, agent = false) {
  const destination = agent ? `${connectPath}?mode=agent` : connectPath;
  return signedIn ? destination : entryPath("/signup", destination);
}
