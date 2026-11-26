import snapshot from "../../../../contracts/openapi.json";
import { apiReferencePath } from "./api-reference-path";
import { modelRequest, sourceOperations } from "./api-source-reference";
import { schemaNotes } from "./api-schema-notes";
import { methods, type HttpMethod, type Operation, type ResponseSchema, type Schema } from "./api-schema-types";

const contract = snapshot as {
  openapi: string;
  paths: Record<string, Partial<Record<HttpMethod, Operation>>>;
  components: {
    schemas: Record<string, unknown>;
    responses: Record<string, ResponseSchema>;
    securitySchemes: Record<string, { description?: string }>;
  };
};

export type ReferenceOperation = {
  method: HttpMethod;
  path: string;
  href: string;
  group: string;
  operation: Operation;
  source?: string;
  auth?: string;
  related?: { label: string; href: string };
};

export const contractVersion = contract.openapi;

export const referenceGroups = [
  "Catalog and quotes", "Generation and chat", "Requests and artifacts", "Projects and access",
  "Routing and providers", "Usage and exports", "Webhooks and connectors", "Accounts and sessions",
  "MCP and OAuth", "Service and operations",
];

function group(operation: Operation, path: string): string {
  if (operation.tags?.includes("media") || operation.tags?.includes("chat")) return "Generation and chat";
  if (/^\/v1\/(catalog|quotes|schemas)\b|\/reviews\b/.test(path)) return "Catalog and quotes";
  if (/webhook|connector-jobs/.test(path)) return "Webhooks and connectors";
  if (/^\/v1\/(operations|setup|rate-limits)\b/.test(path)) return "Service and operations";
  if (/^\/v1\/(usage|exports|archives)\b|\/(settlements|client-budgets)\b/.test(path)) return "Usage and exports";
  if (/^\/v1\/(providers|provider-|compute-providers|routing-policy)\b|\/providers$/.test(path)) return "Routing and providers";
  if (/^\/v1\/projects(\/|$)/.test(path) && !path.includes("/requests/")) return "Projects and access";
  if (/^\/v1\/(model-requests|requests)\b|\/requests\//.test(path)) return "Requests and artifacts";
  if (/^\/(mcp|\.well-known)\b/.test(path)) return "MCP and OAuth";
  if (/^\/v1\/(auth|accounts|user|invitations|files)\b/.test(path)) return "Accounts and sessions";
  return "Service and operations";
}

function entry(method: HttpMethod, path: string, operation: Operation, source?: string, auth?: string): ReferenceOperation {
  const notes = schemaNotes[`${method} ${path}`];
  return {
    method, path, source, auth, href: apiReferencePath(method, path), group: group(operation, path),
    operation: notes?.enrich?.(operation) ?? operation, related: notes?.related,
  };
}

const primary = Object.entries(contract.paths).flatMap(([path, item]) =>
  methods.flatMap((method) => {
    const operation = item[method];
    return operation ? [entry(method, path, operation)] : [];
  }),
);

function modelAlias(method: HttpMethod, suffix: string, adapt: (operation: Operation) => Operation = (operation) => operation) {
  const operation = primary.find((item) => item.method === method && item.path === "/v1/requests" + suffix)?.operation;
  if (!operation) throw new Error("Missing native contract for model request route");
  return entry(method, "/v1/model-requests" + suffix, adapt(operation), "crates/bin/api-gateway/src/gateway_demo/handlers.rs");
}

function modelResponse(operation: Operation, schema: Schema, description: string): Operation {
  const current = operation.responses["200"] as ResponseSchema;
  return {
    ...operation, responses: {
      ...operation.responses, 200: { ...current, description, content: { "application/json": { schema } } },
    },
  };
}

const modelAliases = [
  modelAlias("post", "", (operation) => {
    const quoted = contract.components.schemas.QuotedSubmission as Schema;
    const fields = { ...quoted.properties };
    delete fields.pinned_offer_id;
    fields.quote_id = { type: ["string", "null"], description: "Optional scoped quote ID." };
    return {
      ...operation, summary: "Submit through the model request interface",
      description: "Supply Idempotency-Key and the request input. Include quote_id to require a reviewed estimate. Follow the model request URL in Location.",
      requestBody: { required: true, content: { "application/json": {
        schema: { ...quoted, required: ["input"], properties: fields },
      } } },
    };
  }),
  modelAlias("get", "", (operation) => modelResponse(operation, {
    type: "object", properties: {
      requests: { type: "array", items: modelRequest }, next_after: { type: ["string", "null"] },
    }, required: ["requests", "next_after"],
  }, "A page of model request summaries.")),
  modelAlias("get", "/{id}", (operation) => modelResponse(operation, modelRequest, "Current model request state.")),
  modelAlias("post", "/{id}/cancel", (operation) => modelResponse(operation, modelRequest, "Canceled model request.")),
  modelAlias("get", "/{id}/result"),
  modelAlias("get", "/{id}/artifacts/{slot}"),
];

const documentedRoutes = new Set(primary.map(({ method, path }) => method + " " + path.replace(/\{[^}]+\}/g, "{}")));
const supplemental = [
  ...sourceOperations.map(({ method, path, operation, source, auth }) => entry(method, path, operation, source, auth)),
  ...modelAliases,
].filter(({ method, path }) => !documentedRoutes.has(method + " " + path.replace(/\{[^}]+\}/g, "{}")));

export const operations = [
  ...primary,
  ...supplemental,
].sort((a, b) => a.path.localeCompare(b.path) || methods.indexOf(a.method) - methods.indexOf(b.method));

export function authentication(operation: Operation) {
  return operation.security?.map((alternatives) => Object.keys(alternatives).map((name) =>
    contract.components.securitySchemes[name]?.description ?? name,
  ).join(" and ") || "Public access") ?? [];
}

export function resolveReference<T>(pointer: string) {
  if (!pointer.startsWith("#/")) return undefined;
  let value: unknown = snapshot;
  for (const key of pointer.slice(2).split("/")) {
    const name = key.replaceAll("~1", "/").replaceAll("~0", "~");
    value = value && typeof value === "object" && Object.hasOwn(value, name) ? (value as Record<string, unknown>)[name] : undefined;
  }
  return value as T | undefined;
}
