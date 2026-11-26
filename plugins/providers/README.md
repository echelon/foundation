# TokenCraft providers

| Directory | When to use it |
| --- | --- |
| [Inference](inference/README.md) | Run a plugin that translates TokenCraft requests to a vendor API. Includes fal, Kinovi, and OpenRouter. |
| [Compute](compute/README.md) | Bind a cell's chat offer to OpenAI-compatible endpoints you operate and configure endpoint selection and fallback. |

Both start with a catalog offer. Its `local_binding` selects an operator-configured entry. A queue binding points to an inference plugin. A chat binding points to an inference plugin or defines a compute stack. TokenCraft runs admission, request state, and billing in the gateway and workers.

Use the [queued media protocol](../../docs/provider-connectors.md#queued-media-protocol-version-1) to build your own media connector.
