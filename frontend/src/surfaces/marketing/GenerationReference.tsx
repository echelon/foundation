import { useEffect } from "react";
import { ArrowLeft, ArrowRight } from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { CodeTabs } from "@/components/CodeBlock";
import { API_BASE_URL } from "@/lib/api";
import { MODEL_BY_SLUG } from "@/lib/catalog";
import { commonFields, modelContract, modelsFor, primitiveById, primitives, scopedMediaExample, type GenerationField } from "@/lib/generation-contract";
import { demoEnabled } from "@/lib/gateway";
import "./docs.css";

export default function GenerationReference() {
  const { primitive, model } = useParams();
  const referenced = model ? modelContract(model) : undefined;
  const catalogModel = model && Object.hasOwn(MODEL_BY_SLUG, model) ? MODEL_BY_SLUG[model] : undefined;
  const id = referenced?.profile.primitive ?? primitiveById(primitive ?? catalogModel?.modality ?? "");
  const contract = id ? primitives[id] : undefined;
  const text = primitive === "text" || catalogModel?.modality === "chat";
  const title = referenced?.label ?? catalogModel?.name ?? (text ? "Text" : contract?.label) ?? "Schema not found";

  useEffect(() => {
    const previous = document.title;
    document.title = `${title} API schema | TokenCraft`;
    window.scrollTo(0, 0);
    return () => { document.title = previous; };
  }, [title]);

  const available = model ? Boolean(referenced) : contract?.status === "available";
  const endpoint = model ? `/v1/models/${model}/generate` : `/v1/${id}`;
  const example = referenced?.example ?? contract?.example;
  const projectExample = demoEnabled && !model ? scopedMediaExample(id) : undefined;
  const fields = referenced?.fields ?? contract?.fields;
  const list = !model && id ? modelsFor(id) : [];
  const shared = commonFields.filter(({ name }) => model ? name !== "model" : available || name === "model");
  const rules = referenced?.profile.rules ?? contract?.rules;

  return (
    <article className="docs-schema" aria-labelledby="schema-title">
      <nav className="docs-schema-crumb" aria-label="Breadcrumb">
        <Link to="/docs#model-and-job-guide"><ArrowLeft size={15} aria-hidden /> API documentation</Link>
        {model && id && <Link to={`/docs/primitives/${id}`}>{contract?.label} primitive</Link>}
      </nav>
      <header className="docs-schema-header">
        <p className="docs-schema-status">{available || projectExample ? "Generation API" : text ? "Text API" : contract ? "Schema preview" : "API reference"}</p>
        <h1 id="schema-title">{title}</h1>
        {contract && <p>{referenced?.profile.detail ?? contract.summary}</p>}
        {(available || (!model && contract)) && <p className="docs-schema-endpoint"><span>POST</span> <code>{endpoint}</code></p>}
      </header>
      {text ? (
        <>
          <section aria-labelledby="schema-compat">
            <h2 id="schema-compat">Chat completions</h2>
            <p>The local provider-routing profile exposes <code>POST /v1/chat/completions</code> with an OpenAI-compatible chat body.</p>
            {catalogModel && <p>Check the active profile’s catalog for an alias that can execute a text request.</p>}
            {demoEnabled && <Link to="/docs#chat-completions">Chat request examples <ArrowRight size={15} aria-hidden /></Link>}
          </section>
          <section aria-labelledby="schema-fields">
            <h2 id="schema-fields">Request fields</h2>
            <FieldTable fields={primitives.text.fields} />
          </section>
          <section aria-labelledby="schema-constraints">
            <h2 id="schema-constraints">Constraints</h2>
            <ul className="docs-schema-rules">{primitives.text.rules.map((rule) => <li key={rule}>{rule}</li>)}</ul>
          </section>
        </>
      ) : contract ? (
        <>
          {projectExample && <section aria-labelledby="schema-project">
            <h2 id="schema-project">Project key requests</h2>
            <p>With the local launcher, send a project bearer key with <code>requests:submit</code>. Preview the estimate with this body.</p>
            <CodeTabs tabs={[{ label: "JSON", lang: "json", code: JSON.stringify({ ...projectExample, dry_run: true }, null, 2) }]} />
            <p>To submit, remove <code>dry_run</code> and supply an <code>Idempotency-Key</code>. Poll <code>Location</code> with <code>requests:read</code>. Include a <code>quote_id</code> from the preview to require the same estimate.</p>
          </section>}
          {!available && !projectExample && <p className="docs-schema-notice" role="note">{model
            ? "This catalog model has no connected generation adapter. Refer to the primitive for the shared request fields."
            : "The gateway validates this request shape and returns 501 until an offer is connected. No job is created."}</p>}
          {(!projectExample || available) && (!model || referenced) && fields && (
            <>
              <section aria-labelledby="schema-fields">
                <h2 id="schema-fields">{projectExample ? "Account request fields" : "Request fields"}</h2>
                <FieldTable fields={[...fields, ...shared]} />
              </section>
              <section aria-labelledby="schema-constraints">
                <h2 id="schema-constraints">Constraints</h2>
                <ul className="docs-schema-rules">{rules?.map((rule) => <li key={rule}>{rule}</li>)}</ul>
              </section>
              {example && <section aria-labelledby="schema-example">
                <h2 id="schema-example">Example request</h2>
                <CodeTabs tabs={[{ label: "JSON", lang: "json", code: JSON.stringify(example, null, 2) }]} />
              </section>}
            </>
          )}
          {available && <section aria-labelledby="schema-result">
            <h2 id="schema-result">Job response</h2>
            <p>A successful submission returns <code>201 Created</code>. Poll the <code>Location</code> with the same account. The job records its selected model, estimated total or upper estimate in <code>estimated_max_micros</code>, status, and output URLs as they become available.</p>
            <p className="mt-3">For a pinned model, settings outside its capabilities are dropped before pricing. Each dropped field appears in <code>param_adjustments</code> with <code>action: pruned</code>. When choosing a model automatically, TokenCraft looks for one that supports the requested settings.</p>
          </section>}
          {list.length > 0 && <section aria-labelledby="schema-models">
            <h2 id="schema-models">Model schemas</h2>
            <div className="docs-schema-models">{list.map(({ slug, label }) => (
              <Link to={`/docs/models/${slug}`} key={slug}>{label}<ArrowRight size={15} aria-hidden /></Link>
            ))}</div>
          </section>}
          <footer className="docs-schema-footer">
            {model && id && <Link to={`/docs/primitives/${id}`}>{contract.label} schema</Link>}
            {catalogModel && <Link to={`/models/${catalogModel.slug}#api`}>Model catalog page</Link>}
            <a href={`${API_BASE_URL}/v1/schemas/generation`}>Schema registry (JSON)</a>
          </footer>
        </>
      ) : <p>Use the generation guide to find a primitive or model schema.</p>}
    </article>
  );
}

function FieldTable({ fields }: { fields: GenerationField[] }) {
  return (
    <div className="docs-schema-table">
      <table aria-label="Generation request fields">
        <thead><tr><th scope="col">Field</th><th scope="col">Type</th><th scope="col">Rules and allowed values</th></tr></thead>
        <tbody>{fields.map((field) => (
          <tr key={field.name}>
            <th scope="row"><code>{field.name}</code>{field.required && <small>Required</small>}</th>
            <td>{field.type}</td>
            <td>
              {field.detail}
              {field.values && <div className="docs-schema-values">{field.values.map((item) => <code key={item}>{item}</code>)}</div>}
              {field.default !== undefined && <div>Default <code>{field.default}</code></div>}
            </td>
          </tr>
        ))}</tbody>
      </table>
    </div>
  );
}
