export function secureGatewayUrl(baseUrl: string, origin: string) {
  try {
    const url = new URL(baseUrl, origin);
    const loopback = ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
    if (url.protocol !== "https:" && !(url.protocol === "http:" && loopback)) return null;
    if (url.username || url.password || url.search || url.hash) return null;
    return url.href.replace(/\/+$/, "");
  } catch {
    return null;
  }
}

function shellQuote(value: string) {
  return "'" + value.replaceAll("'", "'\\''") + "'";
}

export function shellSetup(baseUrl: string, checkPath: string) {
  const proxyOption = baseUrl.startsWith("http:") ? " --noproxy '*'" : "";
  return `{
  case "$-" in
    *x*) printf 'Turn off shell tracing with set +x, then run setup again.\\n' >&2 ;;
    *)
      printf 'Paste your TokenCraft API key (input hidden): ' >&2
      if IFS= read -rs tokencraft_setup_key </dev/tty && [ -n "$tokencraft_setup_key" ]; then
        printf '\\n' >&2
        if tokencraft_setup_status=$(
          printf 'Authorization: Bearer %s\\n' "$tokencraft_setup_key" |
            curl -q -fsS${proxyOption} --max-time 15 --header @- --output /dev/null --write-out '%{http_code}' ${shellQuote(baseUrl + checkPath)}
        ) && [[ $tokencraft_setup_status == 2[0-9][0-9] ]]; then
          export TOKENCRAFT_BASE_URL=${shellQuote(baseUrl)}
          export TOKENCRAFT_API_KEY="$tokencraft_setup_key"
          printf 'TokenCraft connected. Key set for this terminal session.\\n'
        else
          printf 'Check the gateway and API key, then run setup again.\\n' >&2
        fi
      else
        printf '\\nNo API key was entered.\\n' >&2
      fi
      unset tokencraft_setup_key tokencraft_setup_status
      ;;
  esac
}`;
}

export function agentSetup(baseUrl: string, checkPath: string, docsUrl: string) {
  return [
    `Connect this project to TokenCraft. Gateway: ${baseUrl}. API docs: ${docsUrl}.`,
    "Read TOKENCRAFT_API_KEY from your process environment or secret manager. If it’s missing, ask me to set it through Terminal setup. Keep the key and Authorization header out of chat, source files, and logs.",
    `Send a bearer-authenticated GET to ${baseUrl}${checkPath} without following redirects. Report the HTTP status and suggest the next integration step for this project.`,
  ].join("\n\n");
}
