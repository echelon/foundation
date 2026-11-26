export type Schema = {
  $ref?: string;
  title?: string;
  description?: string;
  type?: string | string[];
  format?: string;
  properties?: Record<string, Schema>;
  required?: string[];
  items?: Schema;
  additionalProperties?: boolean | Schema;
  oneOf?: Schema[];
  anyOf?: Schema[];
  allOf?: Schema[];
  enum?: unknown[];
  const?: unknown;
  default?: unknown;
  example?: unknown;
  examples?: unknown[];
  [keyword: string]: unknown;
};

export type Parameter = {
  name: string;
  in: string;
  description?: string;
  required?: boolean;
  schema?: Schema;
};

export type Content = Record<string, {
  schema?: Schema;
  example?: unknown;
  examples?: Record<string, { summary?: string; value?: unknown }>;
}>;

type Reference = { $ref: string };
export type ResponseSchema = {
  description?: string;
  headers?: Record<string, { description?: string; schema?: Schema }>;
  content?: Content;
};

export type Operation = {
  tags?: string[];
  summary?: string;
  description?: string;
  operationId?: string;
  parameters?: Parameter[];
  security?: Record<string, string[] | undefined>[];
  requestBody?: { content: Content; required?: boolean; description?: string };
  responses: Record<string, ResponseSchema | Reference>;
};

export type SupplementalOperation = {
  method: HttpMethod;
  path: string;
  source: string;
  auth?: string;
  operation: Operation;
};

export const methods = ["get", "post", "put", "patch", "delete", "head", "options", "trace"] as const;
export type HttpMethod = typeof methods[number];
