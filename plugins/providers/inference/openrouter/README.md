# TokenCraft OpenRouter plugin

This local service proxies TokenCraft OpenAI-compatible text chat to OpenRouter. It supports JSON responses and server-sent event streaming. The gateway uses its reviewed catalog rates and the token counts returned by the provider.

## Run locally

Install Node.js 22. Set `OPENROUTER_API_KEY`, `OPENROUTER_MODELS` to a comma-separated allowlist of upstream model IDs, and `TOKENCRAFT_LOCAL_CHAT_KEY_OPENROUTER` to a generated gateway-to-plugin bearer token.

```sh
npm test
npm start
```

The listener defaults to `127.0.0.1:4022`. Set `PORT` to override it. Run the gateway worker on the same host for the sample local binding. Copy the [example binding](bindings.example.json) and [catalog offer](catalog.example.json) into your [TokenCraft profile](../README.md#start-a-plugin). The sample uses `openai/gpt-4.1-mini` to match the ArtCraft reference preset. Add it to `OPENROUTER_MODELS` or choose another allowed model, and replace the sample zero prices with reviewed rates. Set the matching bearer token on the gateway. Keep the OpenRouter API key in the plugin environment.

The proxy restricts requests to its configured models and forwards standard chat fields. The loopback test covers bearer auth, model allowlisting, JSON, and SSE. [OpenRouter chat completions documentation](https://openrouter.ai/docs/api/api-reference/chat/send-chat-completion-request)

## Rotate the gateway service token

Set `TOKENCRAFT_LOCAL_CHAT_KEY_OPENROUTER` on the plugin to the new token. Set `TOKENCRAFT_PREVIOUS_SERVICE_TOKEN` to the current token and `TOKENCRAFT_PREVIOUS_SERVICE_TOKEN_EXPIRES_AT` to the end of the overlap in UTC, using `YYYY-MM-DDTHH:MM:SSZ`. Start the plugin with these settings, then roll the gateway processes with the new token in the existing binding environment variable.

The plugin stops accepting the previous token at the deadline. Remove the two rotation settings once the gateway rollout is complete. Keep the binding name, origin, and revision stable so outstanding requests keep their connector identity.

## Use organization and project keys

For a tenant-key binding, set `OPENROUTER_TENANT_KEYS=true` and leave `OPENROUTER_API_KEY` unset. Use an independent service bearer token of at least 32 characters in `TOKENCRAFT_LOCAL_CHAT_KEY_OPENROUTER`. Set `tenant_keys: true` and a `revision` on the matching gateway binding. Keep this listener private. If proxying across hosts, terminate TLS and omit both authentication headers from request logs.

After validating the service bearer, the plugin takes an OpenRouter API key from `X-TokenCraft-Upstream-Key` and sends it to the fixed OpenRouter endpoint as its bearer credential. Save or rotate keys through TokenCraft’s organization or project credential API. [OpenRouter API key format](https://openrouter.ai/docs/api_reference/authentication)
