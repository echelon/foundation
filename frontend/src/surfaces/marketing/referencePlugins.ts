import type { Modality } from "@/surfaces/requests/modelOptions";

export const catalogIdentifier = /^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$/;

type QueueBinding = { origin: string; revision: string; token_env: string };
type ChatBinding = { endpoint: string; key_env: string };
type PluginSetup = { kinds: readonly Modality[] } & (
  | { protocol: "queue"; binding: QueueBinding }
  | { protocol: "chat"; binding: ChatBinding }
);

// Keep these defaults aligned with the independent plugins' bindings.example.json files.
const plugins: Record<string, PluginSetup> = {
  fal: {
    kinds: ["image", "video", "audio", "mesh"], protocol: "queue",
    binding: { origin: "http://127.0.0.1:4021/", revision: "v1", token_env: "TOKENCRAFT_CONNECTOR_TOKEN_FAL" },
  },
  kinovi: {
    kinds: ["video"], protocol: "queue",
    binding: { origin: "http://127.0.0.1:4023/", revision: "v1", token_env: "TOKENCRAFT_CONNECTOR_TOKEN_KINOVI" },
  },
  openrouter: {
    kinds: ["text"], protocol: "chat",
    binding: { endpoint: "http://127.0.0.1:4022/v1/chat/completions", key_env: "TOKENCRAFT_LOCAL_CHAT_KEY_OPENROUTER" },
  },
};

export function referencePlugin(id: string): PluginSetup | undefined {
  return Object.hasOwn(plugins, id) ? plugins[id] : undefined;
}

export function bindingTemplates(offers: { binding: string; plugin: string; name: string }[]) {
  const aliases = new Map<string, string>();
  const queue: (QueueBinding & { name: string })[] = [];
  const chat: (ChatBinding & { name: string })[] = [];
  for (const offer of offers) {
    const name = offer.binding.trim();
    if (!catalogIdentifier.test(name)) throw new Error(`Enter a valid connector binding for ${offer.name}.`);
    const setup = referencePlugin(offer.plugin);
    if (!setup) throw new Error(`Choose a supported plugin for ${offer.name}.`);
    const owner = aliases.get(name);
    if (owner === offer.plugin) continue;
    if (owner) throw new Error(`Use distinct binding names for different plugins. “${name}” is assigned to both.`);
    aliases.set(name, offer.plugin);
    if (setup.protocol === "queue") queue.push({ name, ...setup.binding });
    else chat.push({ name, ...setup.binding });
  }
  if (chat.length > 16) throw new Error("Use up to 16 chat bindings for this gateway.");
  return { queue: { version: 1, bindings: queue }, chat: { version: 1, bindings: chat } };
}
