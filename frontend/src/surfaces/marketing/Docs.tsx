import { Fragment, useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { ArrowRight, ArrowUpRight, BookOpen, Check, ChevronDown, Code2, Copy, Link2, List, MessageCircle, Send, Terminal, X } from "lucide-react";
import { Link } from "react-router-dom";
import { CodeBlock } from "@/components/CodeBlock";
import { API_BASE_URL } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { demoEnabled } from "@/lib/gateway";
import { entryPath, getStartedPath } from "@/lib/onboarding";
import { answerDocsQuestion, type DocsReply } from "./docs-assistant";
import { buildDocs, docsDescription, docsMarkdown, docsTitle, endpointId, type Docs as DocsData, type DocsLink, type DocsSection, type Snippet, type Topic } from "./docs-content";
import "./docs.css";

type ManualCopy = (value: string) => void;
const groups = ["Get started", "Build", "Reference", "Advanced"];

export default function Docs() {
  const { user } = useAuth();
  const keyPath = demoEnabled ? getStartedPath(Boolean(user)) : user ? "/dashboard/keys" : entryPath("/signup", "/dashboard/keys");
  const docs = useMemo(() => buildDocs({ baseUrl: API_BASE_URL, demo: demoEnabled, keyPath }), [keyPath]);
  const active = useActiveSection(docs.sections);
  const [manual, setManual] = useState("");
  const manualDialog = useRef<HTMLDialogElement>(null);
  const manualText = useRef<HTMLTextAreaElement>(null);
  const chatDialog = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const previousTitle = document.title;
    document.title = "Docs | TokenCraft";
    return () => { document.title = previousTitle; };
  }, []);

  useEffect(() => {
    if (manual && !manualDialog.current?.open) {
      manualDialog.current?.showModal();
      manualText.current?.select();
    }
  }, [manual]);

  function copyContext() {
    return docsMarkdown(docs, new URL(window.location.pathname, window.location.origin));
  }

  function openChat() {
    chatDialog.current?.showModal();
  }

  return (
    <div className="docs-shell">
      <a className="docs-skip" href="#docs-content">Skip to documentation</a>
      <aside className="docs-sidebar">
        <details className="docs-mobile-toc">
          <summary><List size={17} aria-hidden /> On this page <ChevronDown size={16} aria-hidden /></summary>
          <TableOfContents sections={docs.sections} active={active} onNavigate={(event) => {
            event.currentTarget.closest("details")?.removeAttribute("open");
          }} />
        </details>
        <div className="docs-desktop-toc">
          <div className="docs-sidebar-title"><BookOpen size={18} aria-hidden /> Documentation</div>
          <TableOfContents sections={docs.sections} active={active} />
        </div>
      </aside>

      <article className="docs-content" id="docs-content" tabIndex={-1} aria-labelledby="docs-title">
        <div className="docs-toolbar">
          <div className="docs-breadcrumb"><Link to="/">TokenCraft</Link><span aria-hidden>/</span><span>Docs</span></div>
          <div className="docs-actions">
            <CopyButton label="Copy context" value={copyContext} onManual={setManual} />
            <button className="docs-button docs-assistant-button" type="button" onClick={openChat}><MessageCircle size={16} aria-hidden /> Ask docs</button>
          </div>
        </div>

        <header className="docs-intro">
          <h1 id="docs-title">{docsTitle}</h1>
          <p>{docsDescription}</p>
        </header>

        {docs.sections.map((section) => (
          <section className="docs-section" id={section.id} key={section.id} tabIndex={-1} aria-labelledby={`${section.id}-title`}>
            <h2 className="docs-section-title" id={`${section.id}-title`}>
              <a className="docs-heading-link" href={`#${section.id}`}>{section.title}</a>
            </h2>
            {section.id === "quickstart" ? (
              <ol className="docs-steps">
                {section.topics.map((topic, index) => (
                  <li key={topic.id} id={topic.id} tabIndex={-1}>
                    <span className="docs-step-number" aria-hidden>{index + 1}</span>
                    <TopicContent topic={topic} onManual={setManual} />
                  </li>
                ))}
              </ol>
            ) : (
              <div className="docs-topics">
                {section.topics.map((topic) => (
                  <div className="docs-topic" key={topic.id} id={topic.id} tabIndex={-1}><TopicContent topic={topic} onManual={setManual} /></div>
                ))}
              </div>
            )}
          </section>
        ))}

        <footer className="docs-next">
          <div><h2>Need help with a step?</h2><p>Ask about a command, an endpoint, or an error from this guide.</p></div>
          <button className="docs-button docs-assistant-button" type="button" onClick={openChat}><MessageCircle size={16} aria-hidden /> Ask docs</button>
        </footer>
      </article>

      <DocsChat dialog={chatDialog} docs={docs} />
      <dialog className="docs-dialog docs-copy-dialog" ref={manualDialog} aria-labelledby="docs-copy-title" onClose={() => setManual("")}
        onClick={(event) => { if (event.target === event.currentTarget) event.currentTarget.close(); }}>
        <div className="docs-dialog-header">
          <h2 id="docs-copy-title">Copy manually</h2>
          <CloseButton onClick={() => manualDialog.current?.close()} />
        </div>
        <p>Clipboard access is unavailable. The text is ready to select and copy.</p>
        <textarea ref={manualText} readOnly value={manual} aria-label="Text to copy" />
        <button className="docs-button" type="button" onClick={() => manualText.current?.select()}>Select all</button>
      </dialog>
    </div>
  );
}

function useActiveSection(sections: DocsSection[]) {
  const [active, setActive] = useState(sections[0].id);
  const initialFragmentHandled = useRef(false);

  useEffect(() => {
    const targets = sections.map(({ id }) => document.getElementById(id));
    let frame: number | undefined;
    const update = () => {
      frame = undefined;
      const cutoff = Math.min(220, window.innerHeight * 0.3);
      let current = targets[0];
      for (const target of targets) {
        if (target && target.getBoundingClientRect().top <= cutoff) current = target;
      }
      setActive(current?.id ?? sections[0].id);
    };
    const schedule = () => {
      if (frame === undefined) frame = requestAnimationFrame(update);
    };
    // This route is lazy-loaded, so the initial fragment may precede its target.
    if (!initialFragmentHandled.current) {
      initialFragmentHandled.current = true;
      try {
        const target = document.getElementById(decodeURIComponent(window.location.hash.slice(1)));
        target?.scrollIntoView({ behavior: "instant" });
      } catch {
        // Ignore fragments with invalid URL encoding.
      }
    }
    update();
    window.addEventListener("scroll", schedule, { passive: true });
    window.addEventListener("resize", schedule);
    return () => {
      window.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
      if (frame !== undefined) cancelAnimationFrame(frame);
    };
  }, [sections]);

  return active;
}

function TableOfContents({ sections, active, onNavigate }: {
  sections: DocsSection[];
  active: string;
  onNavigate?: (event: React.MouseEvent<HTMLAnchorElement>) => void;
}) {
  return (
    <nav className="docs-toc" aria-label="Table of contents">
      {groups.map((group) => (
        <div className="docs-toc-group" key={group}>
          <p>{group}</p>
          <ul>
            {sections.filter((section) => section.group === group).map(({ id, title }) => (
              <li key={id}>
                <a href={`#${id}`} aria-current={active === id ? "location" : undefined} onClick={onNavigate}>{title}</a>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  );
}

function InlineText({ text }: { text: string }) {
  return text.split("`").map((part, index) => index % 2 ? <code key={index}>{part}</code> : <Fragment key={index}>{part}</Fragment>);
}

function TopicContent({ topic, onManual }: { topic: Topic; onManual: ManualCopy }) {
  return (
    <div className="docs-topic-body">
      <h3 className="docs-topic-title">
        <a className="docs-heading-link" href={`#${topic.id}`}>{topic.title}</a>
      </h3>
      {topic.paragraphs.map((paragraph) => <p key={paragraph}><InlineText text={paragraph} /></p>)}
      {topic.example && <Example snippet={topic.example} onManual={onManual} />}
      {topic.links && <div className="docs-links">{topic.links.map((link) => <Hotlink key={link.href} link={link} />)}</div>}
      {topic.endpoints && <EndpointTable topic={topic} />}
    </div>
  );
}

function Hotlink({ link }: { link: DocsLink }) {
  if (link.href.startsWith("#")) return <a className="docs-hotlink" href={link.href}>{link.label}<ArrowRight size={15} aria-hidden /></a>;
  if (link.external) return (
    <a className="docs-hotlink" href={link.href} target="_blank" rel="noopener noreferrer">{link.label}<ArrowUpRight size={15} aria-hidden /></a>
  );
  return <Link className="docs-hotlink" to={link.href}>{link.label}<ArrowUpRight size={15} aria-hidden /></Link>;
}

function EndpointTable({ topic }: { topic: Topic }) {
  return (
    <div className="docs-endpoints">
      <table>
        <caption className="sr-only">{topic.title} endpoints</caption>
        <thead><tr><th scope="col">Endpoint</th><th scope="col">Behavior</th></tr></thead>
        <tbody>
          {topic.endpoints?.map(([method, path, summary, doc]) => {
            const id = endpointId(method, path);
            const label = <>
              <span className={`docs-method docs-method-${method.toLowerCase()}`}>{method}</span>
              <code>{path}</code>
            </>;
            return (
              <tr id={id} key={id}>
                <th scope="row">
                  {doc
                    ? <Link className="docs-endpoint-link" to={doc} aria-label={`Documentation for ${method} ${path}`}>{label}<ArrowRight size={13} aria-hidden /></Link>
                    : <span className="docs-endpoint-link">{label}</span>}
                </th>
                <td>{summary}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function Example({ snippet, onManual }: { snippet: Snippet; onManual: ManualCopy }) {
  return (
    <div className="docs-code">
      <div className="docs-code-header">
        <span>{snippet.lang === "bash" ? <Terminal size={16} aria-hidden /> : <Code2 size={16} aria-hidden />}{snippet.label}</span>
        <CopyButton label="Copy" accessibleLabel={`Copy ${snippet.label.toLowerCase()} example`} value={() => snippet.code} onManual={onManual} />
      </div>
      <CodeBlock code={snippet.code} lang={snippet.lang} className="docs-code-body" />
    </div>
  );
}

function CopyButton({ label, accessibleLabel, value, onManual }: { label: string; accessibleLabel?: string; value: () => string; onManual: ManualCopy }) {
  const [copied, setCopied] = useState(false);
  const timeout = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timeout.current), []);

  async function copy() {
    const content = value();
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      clearTimeout(timeout.current);
      timeout.current = setTimeout(() => setCopied(false), 2100);
    } catch {
      onManual(content);
    }
  }

  return (
    <button type="button" className="docs-button docs-copy-button" onClick={() => void copy()} aria-label={accessibleLabel}>
      {copied ? <Check size={16} aria-hidden /> : <Copy size={16} aria-hidden />}
      <span aria-live="polite">{copied ? "Copied" : label}</span>
    </button>
  );
}

function CloseButton({ onClick }: { onClick: () => void }) {
  return <button className="docs-close" type="button" onClick={onClick} aria-label="Close"><X size={20} aria-hidden /></button>;
}

function DocsChat({ dialog, docs }: { dialog: React.RefObject<HTMLDialogElement | null>; docs: DocsData }) {
  const [question, setQuestion] = useState("");
  const [turns, setTurns] = useState<{ question: string; answer: DocsReply }[]>([]);
  const transcript = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const prompts = ["How do I get an API key?", "Show me the setup steps", "Why am I getting a 401?"];

  useEffect(() => {
    const log = transcript.current;
    const latest = log?.lastElementChild;
    if (log && latest && turns.length) {
      log.scrollTop += latest.getBoundingClientRect().top - log.getBoundingClientRect().top - 12;
    }
  }, [turns]);

  function ask(value: string) {
    const clean = value.trim();
    if (!clean) return;
    setTurns((current) => [...current, { question: clean, answer: answerDocsQuestion(docs, clean, current.at(-1)?.answer) }]);
    setQuestion("");
    input.current?.focus();
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    ask(question);
  }

  return (
    <dialog className="docs-dialog docs-chat" ref={dialog} aria-labelledby="docs-chat-title" onClick={(event) => {
      if (event.target === event.currentTarget) event.currentTarget.close();
    }}>
      <div className="docs-chat-frame">
        <div className="docs-dialog-header">
          <div className="docs-chat-brand"><MessageCircle size={20} aria-hidden /></div>
          <div><h2 id="docs-chat-title">Ask docs</h2><p>Answers from the TokenCraft guide</p></div>
          <CloseButton onClick={() => dialog.current?.close()} />
        </div>
        <div className="docs-transcript" ref={transcript} role="log" aria-label="Conversation" aria-live="polite">
          {!turns.length && <div className="docs-chat-intro">
            <p>What are you setting up?</p>
            <div className="docs-prompts">
              {prompts.map((prompt) => <button type="button" key={prompt} onClick={() => ask(prompt)}>{prompt}<ArrowUpRight size={15} aria-hidden /></button>)}
            </div>
          </div>}
          {turns.map(({ question: asked, answer }, index) => (
            <div className="docs-turn" key={index}>
              <p className="docs-question"><span className="sr-only">You: </span>{asked}</p>
              <div className="docs-answer">
                <span className="sr-only">Docs assistant: </span>
                {answer.passages.map(({ title, paragraphs, example }, passageIndex) => (
                  <div className="docs-passage" key={passageIndex}>
                    {title && <h3>{title}</h3>}
                    {paragraphs.map((paragraph) => <p key={paragraph}><InlineText text={paragraph} /></p>)}
                    {example && <CodeBlock code={example.code} lang={example.lang} className="docs-chat-code" />}
                  </div>
                ))}
                <div className="docs-chat-sources" aria-label="Sources">
                  {answer.sources.map(({ id, label, href }) => {
                    const text = <><Link2 size={14} aria-hidden />{label}</>;
                    return href
                      ? <Link key={id} to={href} onClick={() => dialog.current?.close()}>{text}</Link>
                      : <a key={id} href={`#${id}`} onClick={() => dialog.current?.close()}>{text}</a>;
                  })}
                </div>
              </div>
            </div>
          ))}
        </div>
        <form className="docs-chat-form" onSubmit={submit}>
          <label className="sr-only" htmlFor="docs-question">Ask a setup question</label>
          <div>
            <input id="docs-question" ref={input} autoFocus value={question} maxLength={500} placeholder="Ask a setup question…" onChange={(event) => setQuestion(event.target.value)} />
            <button className="docs-button docs-primary" type="submit" aria-label="Send question" disabled={!question.trim()}><Send size={18} aria-hidden /></button>
          </div>
          <p>Questions stay in this browser. Keep API keys out of chat.</p>
        </form>
      </div>
    </dialog>
  );
}
