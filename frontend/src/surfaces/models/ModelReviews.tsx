import { useEffect, useState, type FormEvent } from "react";
import { Link, useLocation } from "react-router-dom";
import { Button, Card, Select, Textarea } from "@/components/ui";
import { API_BASE_URL, ApiError } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { errorMessage, gatewayRequest } from "@/lib/gateway";
import { entryPath } from "@/lib/onboarding";

type MyReview = { rating: number; review_text: string | null };
type PublicReview = { rating: number; review_text: string; updated_at: string };
type ReviewPage = {
  rating_count: number;
  average_rating: number | null;
  reviews: PublicReview[];
  next_before: number | null;
};

const maxReviewBytes = 1200;
const json = { "Content-Type": "application/json" };
const issueClass = "text-sm text-rose-600 dark:text-rose-300";

function publicPage(path: string, signal?: AbortSignal) {
  return gatewayRequest<ReviewPage>(path, { credentials: "omit", signal });
}

export function ModelReviews({ slug }: { slug: string }) {
  const { user, loading } = useAuth();
  const location = useLocation();
  const path = `/v1/models/${encodeURIComponent(slug)}/reviews`;
  const [page, setPage] = useState<ReviewPage>();
  const [revision, setRevision] = useState(0);
  const [issue, setIssue] = useState("");
  const [unavailable, setUnavailable] = useState(false);
  const [paging, setPaging] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    setIssue("");
    publicPage(path, controller.signal).then((current) => {
      if (!controller.signal.aborted) setPage(current);
    }).catch((error) => {
      if (controller.signal.aborted) return;
      if (error instanceof ApiError && error.status === 404) setUnavailable(true);
      else setIssue(errorMessage(error));
    });
    return () => controller.abort();
  }, [path, revision]);

  async function nextPage() {
    if (page?.next_before == null || paging) return;
    setPaging(true);
    setIssue("");
    try {
      const next = await publicPage(`${path}?before=${page.next_before}`);
      setPage((current) => current?.next_before === page.next_before
        ? { ...next, reviews: [...current.reviews, ...next.reviews] }
        : current);
    } catch (error) {
      setIssue(errorMessage(error));
    } finally {
      setPaging(false);
    }
  }

  if (unavailable) return null;

  return (
    <Card className="mb-8 space-y-4 p-4 sm:p-5">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <h2 className="font-display text-lg font-semibold">Community reviews</h2>
          {page && <p className="mt-1 text-sm text-muted" aria-live="polite">
            {page.average_rating === null ? "No ratings yet." : (
              <><span className="font-semibold text-fg">{page.average_rating.toFixed(2)} / 10</span>
                {" · "}{page.rating_count.toLocaleString()} {page.rating_count === 1 ? "rating" : "ratings"}</>
            )}
          </p>}
        </div>
        {page && <a className="text-xs font-medium text-brand hover:underline" href={`${API_BASE_URL}${path}`}>Public API</a>}
      </div>
      {issue && <p role="alert" className={issueClass}>{issue}</p>}
      {page && user && (
        <ReviewForm key={user.id} path={path} onChange={() => setRevision((value) => value + 1)} />
      )}
      {page && !loading && !user && <Link
        className="inline-block text-sm font-medium text-brand hover:underline"
        to={entryPath("/login", `${location.pathname}${location.search}${location.hash}`)}
      >Sign in to rate this model</Link>}
      {page && page.reviews.length > 0 && (
        <ul className="divide-y divide-border border-t border-border">
          {page.reviews.map((review, index) => (
            <li key={index} className="space-y-1 py-3">
              <div className="flex flex-wrap items-baseline gap-x-3 text-xs text-muted">
                <span className="font-semibold text-fg">{review.rating} / 10</span>
                <time dateTime={review.updated_at}>{new Date(review.updated_at).toLocaleDateString()}</time>
              </div>
              <p className="whitespace-pre-wrap break-words text-sm">{review.review_text}</p>
            </li>
          ))}
        </ul>
      )}
      {page?.next_before != null && <Button type="button" size="sm" variant="outline" disabled={paging} onClick={nextPage}>Load more reviews</Button>}
    </Card>
  );
}

function ReviewForm({ path, onChange }: { path: string; onChange: () => void }) {
  const mine = `${path}/me`;
  const [saved, setSaved] = useState<MyReview | null>();
  const [rating, setRating] = useState("");
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [issue, setIssue] = useState("");
  const [message, setMessage] = useState("");
  const [reload, setReload] = useState(0);

  function apply(review: MyReview | null) {
    setSaved(review);
    setRating(review ? String(review.rating) : "");
    setText(review?.review_text ?? "");
  }

  useEffect(() => {
    const controller = new AbortController();
    gatewayRequest<MyReview | null>(mine, { signal: controller.signal }).then((review) => {
      if (!controller.signal.aborted) apply(review);
    }).catch((error) => {
      if (!controller.signal.aborted) setIssue(errorMessage(error));
    });
    return () => controller.abort();
  }, [mine, reload]);

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy || !rating) return;
    await mutate("PUT");
  }

  async function mutate(method: "PUT" | "DELETE") {
    setBusy(true);
    setIssue("");
    setMessage("");
    try {
      if (method === "PUT") {
        const review = await gatewayRequest<MyReview>(mine, {
          method, headers: json, body: JSON.stringify({ rating: Number(rating), review_text: text.trim() || null }),
        });
        apply(review);
        setMessage("Review saved.");
      } else {
        await gatewayRequest<void>(mine, { method });
        apply(null);
        setMessage("Review removed.");
      }
      onChange();
    } catch (error) {
      setIssue(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }

  if (saved === undefined) return issue ? <div className="space-y-2">
    <p role="alert" className={issueClass}>{issue}</p>
    <Button type="button" size="sm" variant="outline" onClick={() => { setIssue(""); setReload((value) => value + 1); }}>Reload your review</Button>
  </div> : null;
  const overLimit = new TextEncoder().encode(text.trim()).length > maxReviewBytes;

  return (
    <form className="space-y-3 border-t border-border pt-4" onSubmit={save}>
      <label className="block max-w-44 space-y-1 text-sm font-medium">
        Your rating
        <Select required value={rating} disabled={busy} onChange={(event) => setRating(event.target.value)}>
          <option value="">Choose 1–10</option>
          {Array.from({ length: 10 }, (_, index) => index + 1).map((value) => <option key={value} value={value}>{value} / 10</option>)}
        </Select>
      </label>
      <label className="block space-y-1 text-sm font-medium">
        What worked well or poorly? <span className="font-normal text-muted">(optional)</span>
        <Textarea value={text} rows={3} maxLength={maxReviewBytes} readOnly={busy} onChange={(event) => setText(event.target.value)} />
      </label>
      <p className="text-xs text-muted">Your rating and text are public without your profile. The Model Advisor may summarize written reviews. Leave out personal or confidential details.</p>
      {overLimit && <p role="alert" className={issueClass}>Use up to 1,200 UTF-8 bytes.</p>}
      {issue && <p role="alert" className={issueClass}>{issue}</p>}
      {message && <p role="status" className="text-sm text-muted">{message}</p>}
      <div className="flex flex-wrap items-center gap-2">
        <Button type="submit" size="sm" disabled={busy || !rating || overLimit}>Save review</Button>
        {saved && <Button type="button" size="sm" variant="ghost" disabled={busy} onClick={() => void mutate("DELETE")}>Remove review</Button>}
      </div>
    </form>
  );
}
