import { MODELS, type Modality } from "@/lib/catalog";
import { gatewayRequest } from "@/lib/gateway";

export type Recommendation = {
  slug: string;
  reason: string;
  tradeoff?: string | null;
  community?: {
    rating_count: number;
    average_rating: number | null;
    written_reviews_sampled: number;
    strengths: string[];
    weaknesses: string[];
  };
};
type Source = { url: string; title: string };
export type Advice = { recommendations: Recommendation[]; sources: Source[]; comparison_slugs?: string[] };

const catalogCandidates = MODELS.map((model) => model.slug);

export const advisorApi = {
  status: (signal: AbortSignal) => gatewayRequest<{ web_research: boolean }>("/v1/model-advisor", { signal }),
  research: (job: string, signal: AbortSignal) => gatewayRequest<Advice>("/v1/model-advisor", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ job, catalog_candidates: catalogCandidates }),
    signal,
  }),
};

const intent: Record<Modality, RegExp> = {
  video: /\b(?:videos?|clips?|footage|animat\w+|b[- ]?roll|timelapse)\b/i,
  image: /\b(?:images?|photos?|portraits?|posters?|illustrat\w+|renders?|logos?)\b/i,
  world: /\b(?:worlds?|environments?|walkthroughs?|explorable|spatial)\b/i,
  "3d": /\b(?:3d|meshes?|mesh|geometry|game assets?|sculpt\w+)\b/i,
  chat: /\b(?:assistant|chat|language|llm|text|code|coding|reasoning|summari\w+|writ\w+|translat\w+)\b/i,
};

const words = (text: string) => new Set(text.toLowerCase().match(/[a-z0-9]{3,}/g) ?? []);

export function catalogAdvice(job: string): Advice {
  const terms = words(job);
  const direction = job.split(/\b(?:into|as the output|output should be)\b/i).at(-1) ?? job;
  const modalities = Object.keys(intent) as Modality[];
  const desired = modalities.filter((modality) => intent[modality].test(direction));
  const signals = desired.length ? desired : modalities.filter((modality) => intent[modality].test(job));
  const budget = /\b(?:cheap\w*|budget|affordable|inexpensive|low[- ]cost|cost[- ]effective)\b/i.test(job);
  const speed = /\b(?:fast\w*|quick\w*|latency|real[- ]time)\b/i.test(job);

  const capabilityMatchWeight = 3;
  const candidates = MODELS.flatMap((model) => {
    const matchingTags = model.tags.filter((tag) => {
      const tagTerms = words(tag);
      return tagTerms.size > 0 && [...tagTerms].every((term) => terms.has(term));
    });
    const nameMatch = job.toLowerCase().includes(model.name.toLowerCase());
    const modalityMatch = signals.includes(model.modality);
    const score = (nameMatch ? 10 : 0) + (modalityMatch ? 4 : 0) + matchingTags.length * capabilityMatchWeight;
    if (score === 0) return [];
    const lowLatency = speed && model.tags.some((tag) => /fast|latency/.test(tag));
    const reason = matchingTags.length
      ? `The catalog lists ${matchingTags.slice(0, 3).join(", ")} among this model’s capabilities.`
      : model.tagline;
    return [{ model, score: score + Number(lowLatency), cost: model.pricing[0]?.usd ?? Infinity, reason }];
  });

  const comparableCosts = budget && new Set(candidates.map(({ model }) => model.modality)).size === 1;
  candidates.sort((left, right) => {
    const difference = right.score - left.score;
    if (difference || !comparableCosts) return difference;
    return left.cost - right.cost;
  });

  const shortlist = candidates.slice(0, 3);
  const best = shortlist[0];
  const peers = best ? shortlist.filter(({ model, score }) =>
    model.modality === best.model.modality && best.score - score <= capabilityMatchWeight) : [];
  return {
    recommendations: shortlist.map(({ model, reason }) => ({ slug: model.slug, reason })),
    sources: [],
    comparison_slugs: peers.length > 1 ? peers.map(({ model }) => model.slug) : [],
  };
}
