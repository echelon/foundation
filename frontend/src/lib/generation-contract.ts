import registry from "../../content/api/generation.json";

type Value = string | number;
export type GenerationField = {
  name: string;
  type: string;
  detail: string;
  required?: boolean;
  default?: string;
  values?: Value[];
};

type Primitive = {
  label: string;
  status: string;
  summary: string;
  fields: GenerationField[];
  rules: string[];
  example: Record<string, unknown>;
};

type Profile = {
  primitive: PrimitiveId;
  fields: string[];
  detail: string;
  rules: string[];
  values?: Record<string, Value[]>;
};

export type PrimitiveId = keyof typeof registry.primitives;
export const primitives: Record<PrimitiveId, Primitive> = registry.primitives;
export const commonFields: GenerationField[] = registry.common_fields;
const modelEntries: Record<string, { label: string; profile: string }> = registry.models;
const profiles = registry.profiles as Record<string, Profile>;

export function primitiveById(id: string): PrimitiveId | undefined {
  if (Object.hasOwn(primitives, id)) return id as PrimitiveId;
}

export function scopedMediaExample(id: PrimitiveId | undefined) {
  const examples: Partial<Record<PrimitiveId, Record<string, unknown>>> = registry.scoped_media_examples;
  return id ? examples[id] : undefined;
}

export function modelContract(slug: string) {
  if (!Object.hasOwn(modelEntries, slug)) return;
  const { label, profile } = modelEntries[slug];
  const spec = profiles[profile];
  const primitive = primitives[spec.primitive];
  const fields = primitive.fields.filter(({ name, required }) => required || spec.fields.includes(name))
    .map((field) => ({ ...field, values: spec.values?.[field.name] ?? field.values }));
  const accepted = new Set([...fields, ...commonFields].map(({ name }) => name));
  const example = Object.fromEntries(Object.entries(primitive.example).filter(([name]) => accepted.has(name)));
  return { label, profile: spec, fields, example };
}

export function modelsFor(primitive: PrimitiveId) {
  return Object.entries(modelEntries)
    .filter(([, entry]) => profiles[entry.profile].primitive === primitive)
    .map(([slug, { label }]) => ({ slug, label }));
}
