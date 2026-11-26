import { endpointId, type Docs, type Snippet, type Topic } from "./docs-content";

type Source = { id: string; label: string; href?: string };
export type DocsReply = {
  passages: { title: string; paragraphs: string[]; example?: Snippet }[];
  sources: Source[];
};

const stopWords = new Set("a an and are can could did do does for from get give have how i in is it me my of on or please should show tell that the this to use using was what when where which will with would you your".split(" "));

function terms(text: string) {
  return new Set((text.toLowerCase().match(/[a-z0-9]+/g) ?? [])
    .filter((word) => !stopWords.has(word))
    .map((word) => word.length > 4 ? word.replace(/ies$/, "y").replace(/s$/, "") : word));
}

const overlap = (search: Set<string>, text: string) =>
  [...terms(text)].filter((word) => search.has(word)).length;

function topicScore(question: string, search: Set<string>, topic: Topic) {
  const aliases = topic.keywords ?? [];
  const phraseMatch = aliases.some((phrase) => phrase.includes(" ") && question.includes(phrase));
  return overlap(search, topic.title) * 3
    + Math.min(8, overlap(search, aliases.join(" ")) * 3)
    + Math.min(3, overlap(search, topic.paragraphs.join(" ")))
    + (phraseMatch ? 6 : 0);
}

function topicReply(topics: Topic[]): DocsReply {
  return {
    passages: topics.map(({ title, paragraphs, example }) => ({ title, paragraphs, example })),
    sources: topics.map(({ id, title }) => ({ id, label: title })),
  };
}

export function answerDocsQuestion(docs: Docs, input: string, previous: DocsReply | undefined): DocsReply {
  const question = input.toLowerCase().trim();
  const topics = docs.sections.flatMap((section) => section.topics);

  // Match exact paths before scoring natural language. Each method has its own hotlink.
  const endpointQuery = question.match(/(?:\b(get|post|put)\s+)?(\/v1\/[a-z0-9_{}\/.-]+)/i);
  if (endpointQuery) {
    const [, verb, rawPath] = endpointQuery;
    const path = rawPath.replace(/[./]+$/, "");
    const endpoints = topics.flatMap((topic) => topic.endpoints ?? []).filter(([method, template]) => {
      if (verb && method.toLowerCase() !== verb) return false;
      const templateParts = template.split("/");
      const pathParts = path.split("/");
      return templateParts.length === pathParts.length && templateParts.every((part, index) =>
        part === pathParts[index] || part.startsWith("{") && /^[a-z0-9_{}.-]+$/i.test(pathParts[index]));
    });
    if (endpoints.length) {
      const sources = endpoints.map(([method, template, , href]) => ({ id: endpointId(method, template), label: `${method} ${template}`, href }));
      if (/\b(how|example|command|curl|headers)\b/.test(question)) {
        const guides = topics.filter((topic) => topic.paragraphs.some((paragraph) =>
          endpoints.some(([method, template]) => paragraph.includes(`\`${method} ${template}\``))));
        if (guides.length) {
          const answer = topicReply(guides.slice(0, 2));
          return { passages: answer.passages, sources: [...answer.sources, ...sources] };
        }
      }
      return {
        passages: endpoints.map(([method, template, summary]) => ({ title: `${method} ${template}`, paragraphs: [summary] })),
        sources,
      };
    }
    return {
      passages: [{ title: "", paragraphs: ["I couldn’t find that endpoint in this version of the guide. Check the endpoint reference for the documented paths."] }],
      sources: docs.sections.filter(({ group }) => group === "Reference").map(({ id, title }) => ({ id, label: title })),
    };
  }

  if (/\b(?:first request|from scratch|setup steps|get started|set ?up tokencraft|how (?:do|can) i set ?up|how do i start|quick ?start)\b/.test(question)
      && !/\b(?:agent|chat|webhook|error|401|403)\b/.test(question)) {
    const quickstart = docs.sections.find(({ id }) => id === "quickstart");
    if (quickstart) return topicReply(quickstart.topics);
  }

  if (previous && /^(?:and )?(?:(?:show|give|send) (?:me )?(?:the |an? )?(?:command|code|example)|what(?:'s| is) (?:the |an? )?(?:command|example)|can i see (?:the |an? )?(?:command|code|example))[?.!]*$/.test(question)) {
    const passages = previous.passages.filter(({ example }) => example);
    if (passages.length) return { passages, sources: previous.sources };
  }

  const search = terms(question);
  const ranked = topics.filter((topic) => topic.paragraphs.length).map((topic) => ({
    topic,
    score: topicScore(question, search, topic),
  })).sort((a, b) => b.score - a.score);
  const best = ranked[0];
  if (best?.score >= 4) {
    const matches = ranked.filter(({ score }) => score >= Math.max(6, best.score * 0.72)).slice(0, 2);
    return topicReply(matches.length ? matches.map(({ topic }) => topic) : [best.topic]);
  }

  const references = topics.filter((topic) => topic.endpoints && overlap(search, topic.title) > 0);
  if (references.length) return {
    passages: [{ title: "", paragraphs: ["These reference sections document the matching operations. Ask about a specific path for its behavior."] }],
    sources: references.map(({ id, title }) => ({ id, label: title })),
  };

  return {
    passages: [{ title: "", paragraphs: ["I couldn’t find an answer in this guide. Try asking about API keys, the connection check, model endpoints, or a specific HTTP error."] }],
    sources: [{ id: "quickstart", label: "Quickstart" }, { id: "troubleshooting", label: "Troubleshooting" }],
  };
}
