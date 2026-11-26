import { useEffect, useState, type ReactNode } from "react";
import { Link, useParams } from "react-router-dom";
import { API_BASE_URL } from "@/lib/api";
import { authentication, contractVersion, operations, referenceGroups, resolveReference, type ReferenceOperation } from "./api-reference-data";
import { methods, type Content, type Parameter, type ResponseSchema, type Schema } from "./api-schema-types";
import "./docs.css";

const sourceUrl = "https://github.com/storytold/tokencraft/blob/912c9d3ec7117966e84950e8acf2b97a303a77d0/";
const contractUrl = import.meta.env.DEV ? API_BASE_URL.replace(/\/$/, "") + "/openapi.json" : "/docs/openapi.json";
const consoleLinks = { guide: "/docs", design: "/design/#api-schema/caller", contract: contractUrl };
type ReferenceLinks = typeof consoleLinks;
const keywords: [string, string][] = [
  ["minimum", "Min"], ["maximum", "Max"], ["exclusiveMinimum", "Above"], ["exclusiveMaximum", "Below"],
  ["minLength", "Min length"], ["maxLength", "Max length"], ["minItems", "Min items"], ["maxItems", "Max items"],
  ["pattern", "Pattern"], ["default", "Default"], ["const", "Value"],
];
const httpMethods = new Set<string>(methods);

export default function ApiReference({ links = consoleLinks }: { links?: ReferenceLinks }) {
  const { method, "*": suffix } = useParams();
  const operation = method ? operations.find((item) => item.method === method && item.path === "/" + (suffix ?? "")) : undefined;

  useEffect(() => {
    const previous = document.title;
    document.title = operation ? `${operation.method.toUpperCase()} ${operation.path} | TokenCraft` : "API reference | TokenCraft";
    window.scrollTo(0, 0);
    return () => { document.title = previous; };
  }, [method, operation]);

  return (
    <article className="docs-schema api-reference" aria-labelledby="api-ref-title">
      <nav className="docs-schema-crumb" aria-label="Breadcrumb">
        <a href={links.guide + "#openapi-reference"}>Documentation</a>
        {method && <Link to="/docs/api">API reference</Link>}
      </nav>
      {!method ? <ReferenceIndex links={links} /> : operation ? <OperationPage key={operation.href} item={operation} links={links} /> : (
        <header className="docs-schema-header">
          <h1 id="api-ref-title">Endpoint not found</h1>
          <p>Browse the <Link to="/docs/api">API reference</Link> to find the operation.</p>
        </header>
      )}
    </article>
  );
}

function ReferenceIndex({ links }: { links: ReferenceLinks }) {
  const [search, setSearch] = useState("");
  const [group, setGroup] = useState("");
  const terms = search.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const matching = operations.filter(({ group: category, method, path, operation }) => {
    const indexText = [category, path, operation.summary, operation.operationId].join(" ").toLowerCase();
    return (!group || category === group) && terms.every((term) =>
      httpMethods.has(term) ? method === term : indexText.includes(term));
  });
  return (
    <>
      <header className="docs-schema-header">
        <h1 id="api-ref-title">API reference</h1>
        <p>Find the request fields, authorization, response formats, and status codes for each gateway operation.</p>
        <div className="docs-links">
          <a href={links.guide + "#api-conventions"}>API conventions</a>
          <a href={links.design}>Backend schema flow</a>
          <a href={links.contract}>OpenAPI JSON</a>
        </div>
      </header>
      <div className="api-reference-filters">
        <label>Search endpoints
          <input type="search" value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Method, path, or operation" />
        </label>
        <label>Category
          <select value={group} onChange={(event) => setGroup(event.target.value)}>
            <option value="">All categories</option>
            {referenceGroups.map((name) => <option key={name}>{name}</option>)}
          </select>
        </label>
      </div>
      <p className="api-reference-count" role="status">{matching.length} {matching.length === 1 ? "operation" : "operations"}</p>
      {referenceGroups.map((name) => {
        const items = matching.filter((item) => item.group === name);
        if (!items.length) return null;
        return (
          <section key={name}>
            <h2>{name}</h2>
            <ul className="api-reference-list">
              {items.map((item) => (
                <li key={item.href}>
                  <Link to={item.href}>
                    <Endpoint item={item} />
                    {item.operation.summary && <span className="api-operation-summary">{summary(item.operation.summary)}</span>}
                  </Link>
                </li>
              ))}
            </ul>
          </section>
        );
      })}
    </>
  );
}

function summary(value: string) {
  return value.split("\n", 1)[0].replace(/\.$/, "");
}

function Endpoint({ item }: { item: ReferenceOperation }) {
  return (
    <span className="api-endpoint">
      <span className={`docs-method docs-method-${item.method}`}>{item.method.toUpperCase()}</span>
      <code>{item.path}</code>
    </span>
  );
}

function OperationPage({ item, links }: { item: ReferenceOperation; links: ReferenceLinks }) {
  const { operation, related, source } = item;
  const auth = item.auth ? [item.auth] : authentication(operation);
  const headline = operation.summary ? summary(operation.summary) : undefined;

  return (
    <>
      <header className="docs-schema-header">
        <p className="docs-schema-status">{item.group}</p>
        <h1 id="api-ref-title">{headline ?? `${item.method.toUpperCase()} ${item.path}`}</h1>
        {headline && <p className="docs-schema-endpoint"><Endpoint item={item} /></p>}
        {operation.description && <p className="api-reference-description">{operation.description}</p>}
        <div className="docs-links">
          {source ? <a href={sourceUrl + source}>Handler source</a> : <a href={sourceUrl + "contracts/openapi.json"}>OpenAPI {contractVersion} snapshot</a>}
          {related && (related.href.startsWith("/docs/api/")
            ? <Link to={related.href}>{related.label}</Link>
            : links === consoleLinks && <Link to={related.href}>{related.label}</Link>)}
        </div>
      </header>

      <section aria-labelledby="api-auth">
        <h2 id="api-auth">Authorization</h2>
        {auth.length > 1 && <p>Use any one of the following credentials.</p>}
        {auth.length > 1 ? (
          <ul className="docs-schema-rules">{auth.map((detail) => <li key={detail}>{detail}</li>)}</ul>
        ) : <p>{auth[0] ?? "No authentication is declared in the contract."}</p>}
      </section>

      <section aria-labelledby="api-input">
        <h2 id="api-input">Request</h2>
        {operation.parameters?.length ? <Parameters values={operation.parameters} /> : null}
        {operation.requestBody ? (
          <div className="api-request-body">
            <h3>Body <small>{operation.requestBody.required ? "Required" : "Optional"}</small></h3>
            {operation.requestBody.description && <p>{operation.requestBody.description}</p>}
            <MediaSchemas content={operation.requestBody.content} />
          </div>
        ) : <p>No request body.</p>}
      </section>

      <section aria-labelledby="api-output">
        <h2 id="api-output">Responses</h2>
        {Object.entries(operation.responses).map(([code, entry]) => {
          const response = "$ref" in entry ? resolveReference<ResponseSchema>(entry.$ref) : entry;
          return (
            <Expandable key={code} label={<><code>{code === "default" ? "Other errors" : code}</code><span>{response?.description}</span></>}
              initialOpen={code.startsWith("2")}>
              {response?.headers && <Parameters values={Object.entries(response.headers).map(([name, value]) => ({
                ...value, name, in: "response header",
              }))} />}
              {response?.content
                ? <MediaSchemas content={response.content} />
                : <p>{code === "204" || code === "304" ? "No response body." : "No response body documented."}</p>}
            </Expandable>
          );
        })}
      </section>
    </>
  );
}

function Parameters({ values }: { values: Parameter[] }) {
  return (
    <div className="docs-schema-table">
      <table>
        <thead><tr><th scope="col">Name</th><th scope="col">Location</th><th scope="col">Type</th><th scope="col">Description</th></tr></thead>
        <tbody>
          {values.map(({ name, in: location, required, schema, description }) => (
            <tr key={location + name}>
              <th scope="row"><code>{name}</code>{required && <small>Required</small>}</th>
              <td>{location}</td>
              <td>{schema ? <SchemaType schema={schema} /> : "—"}</td>
              <td>{description ?? schema?.description}<Constraints schema={schema} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Expandable({ label, children, initialOpen = false }: { label: ReactNode; children: ReactNode; initialOpen?: boolean }) {
  const [open, setOpen] = useState(initialOpen);
  return (
    <details className="api-schema-details" open={open} onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary>{label}</summary>
      {open && <div>{children}</div>}
    </details>
  );
}

function MediaSchemas({ content }: { content: Content }) {
  return (
    <div className="api-schema-media">
      {Object.entries(content).map(([mime, { schema, example, examples }]) => (
        <div key={mime}>
          <h4><code>{mime}</code></h4>
          {schema && <SchemaDetails schema={schema} />}
          {example !== undefined && <Example label="Example" value={example} />}
          {examples && Object.entries(examples).map(([name, entry]) =>
            entry.value !== undefined && <Example key={name} label={entry.summary ?? name} value={entry.value} />)}
        </div>
      ))}
    </div>
  );
}

function Example({ label, value }: { label: string; value: unknown }) {
  return <div className="api-schema-example"><h4>{label}</h4><pre><code>{typeof value === "string" ? value : JSON.stringify(value, null, 2)}</code></pre></div>;
}

function schemaType(schema: Schema): string {
  if (schema.$ref) return schema.$ref.split("/").at(-1) ?? "Reference";
  if (schema.type === "array" && schema.items) return schemaType(schema.items) + "[]";
  if (Array.isArray(schema.type)) return schema.type.join(" | ");
  if (schema.type) return schema.type;
  if (schema.oneOf || schema.anyOf) return "Union";
  if (schema.allOf) return "Combined schema";
  if (schema.properties || schema.additionalProperties) return "object";
  return "JSON value";
}

function SchemaType({ schema }: { schema: Schema }) {
  return <code>{schemaType(schema)}{schema.format ? ` (${schema.format})` : ""}</code>;
}

function Constraints({ schema }: { schema?: Schema }) {
  if (!schema) return null;
  const values = keywords.filter(([key]) => schema[key] !== undefined);
  const hasRules = values.length || schema.enum?.length || schema.additionalProperties === false || schema.uniqueItems;
  return hasRules ? (
    <div className="docs-schema-values">
      {values.map(([key, label]) => <small key={key}>{label} <code>{JSON.stringify(schema[key])}</code></small>)}
      {schema.enum?.map((value) => <code key={JSON.stringify(value)}>{JSON.stringify(value)}</code>)}
      {schema.additionalProperties === false && <small>Unknown fields rejected</small>}
      {Boolean(schema.uniqueItems) && <small>Unique items</small>}
    </div>
  ) : null;
}

function nested(schema: Schema) {
  return Boolean(schema.$ref || schema.properties || schema.items || schema.oneOf || schema.anyOf || schema.allOf ||
    typeof schema.additionalProperties === "object");
}

function SchemaDetails({ schema, ancestors = [] }: { schema: Schema; ancestors?: string[] }) {
  if (schema.$ref) {
    const name = schemaType(schema);
    const target = resolveReference<Schema>(schema.$ref);
    if (!target || ancestors.includes(schema.$ref)) return <p><code>{name}</code></p>;
    return (
      <>
        {schema.description && <p>{schema.description}</p>}
        <SchemaDetails schema={target} ancestors={[...ancestors, schema.$ref]} />
      </>
    );
  }

  const variants = schema.oneOf ?? schema.anyOf ?? schema.allOf;
  const variantTitle = schema.allOf ? "All of" : schema.oneOf ? "One of" : "Any of";
  return (
    <div className="api-schema-shape">
      {schema.description && <p>{schema.description}</p>}
      <p><SchemaType schema={schema} /></p>
      <Constraints schema={schema} />
      {schema.properties && (
        <dl className="api-schema-fields">
          {Object.entries(schema.properties).map(([name, field]) => (
            <div key={name}>
              <dt><code>{name}</code><span>{schema.required?.includes(name) ? "Required" : "Optional"}</span></dt>
              <dd>
                <SchemaType schema={field} />
                {field.description && <p>{field.description}</p>}
                <Constraints schema={field} />
                {nested(field) && <Expandable label="Expand field schema"><SchemaDetails schema={field} ancestors={ancestors} /></Expandable>}
              </dd>
            </div>
          ))}
        </dl>
      )}
      {schema.items && <Expandable label="Array item schema"><SchemaDetails schema={schema.items} ancestors={ancestors} /></Expandable>}
      {typeof schema.additionalProperties === "object" && (
        <Expandable label="Map value schema"><SchemaDetails schema={schema.additionalProperties} ancestors={ancestors} /></Expandable>
      )}
      {variants && <div className="api-schema-union">
        <p>{variantTitle}</p>
        {variants.map((variant, index) => (
          <Expandable key={index} label={variant.title ?? schemaType(variant)}>
            <SchemaDetails schema={variant} ancestors={ancestors} />
          </Expandable>
        ))}
      </div>}
      {schema.example !== undefined && <Example label="Example" value={schema.example} />}
      {schema.examples?.map((value, index) => <Example key={index} label={`Example ${index + 1}`} value={value} />)}
    </div>
  );
}
