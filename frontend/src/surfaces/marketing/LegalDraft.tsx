import { useEffect } from "react";
import { Link } from "react-router-dom";
import { Container } from "@/components/ui";

type LegalDocument = "terms" | "privacy";

export default function LegalDraft({ kind }: { kind: LegalDocument }) {
  const title = kind === "terms" ? "Terms of Service" : "Privacy Notice";

  useEffect(() => {
    const previousTitle = document.title;
    document.title = `${title} | TokenCraft`;
    return () => { document.title = previousTitle; };
  }, [title]);

  return (
    <Container className="py-12 sm:py-16">
      <article className="mx-auto max-w-3xl" aria-labelledby="legal-title">
        <header className="mb-8 border-b border-border pb-6">
          <h1 id="legal-title" className="text-3xl font-semibold tracking-tight sm:text-4xl">{title}</h1>
          <p className="mt-3 text-sm text-muted">Request content policy draft for review · November 25, 2026</p>
        </header>
        <div className="space-y-8 leading-relaxed text-fg [&_section]:space-y-3 [&_section_h2]:text-xl [&_section_h2]:font-semibold">
          {kind === "terms" ? <TermsDraft /> : <PrivacyDraft />}
        </div>
      </article>
    </Container>
  );
}

function TermsDraft() {
  return (
    <>
      <section>
        <h2>Request Content and request history</h2>
        <p>
          Request Content means the inputs you submit to TokenCraft, including prompts and files, and the outputs generated in response. By submitting a request, you authorize us to process, store, and transmit Request Content as needed to fulfill your requests, deliver results, provide request history, secure the service, prevent abuse, and troubleshoot requests. We may also process or retain Request Content as required by law.
        </p>
        <p>
          We keep your inputs and outputs readily available in request history for 30 days after a request finishes. We then retain them in archival storage for up to 365 more days, for a total of up to 395 days after the request finishes. We delete the content when that period ends or sooner if we grant a deletion request, unless the law requires us to retain it longer. Our{" "}
          <Link to="/privacy" className="text-brand underline underline-offset-2">Privacy Notice</Link>{" "}
          explains retention and how to request deletion.
        </p>
      </section>
      <section>
        <h2>Optional model training</h2>
        <p>
          When you enable Model Training, you give TokenCraft permission to use selected Request Content from eligible requests you submit after enabling it to train, fine-tune, and evaluate TokenCraft models. You can withdraw this permission at any time in Settings → Privacy. Turning Model Training off stops future training and evaluation uses of your Request Content. While it is off, we use Request Content only for the service purposes listed above or as required by law. The same request-history retention schedule applies with Model Training on or off.
        </p>
      </section>
    </>
  );
}

function PrivacyDraft() {
  return (
    <>
      <section>
        <h2>What we collect, use, and share</h2>
        <p>
          When you submit a request, we collect the inputs you provide, such as prompts and uploaded files, the outputs generated for you, and request details such as the date, model, status, and usage. We use the inputs and outputs to fulfill requests, display results, provide request history, secure the service, prevent abuse, and troubleshoot requests. We use request details for those purposes and to track usage and billing. When a request uses a hosted model, we send the inputs needed to fulfill it to the model provider. See our [provider disclosures] for details about providers and their roles.
        </p>
      </section>
      <section>
        <h2>How long we keep request content</h2>
        <p>
          Your inputs and outputs are readily available in request history for 30 days after a request finishes. We then keep them in archival storage for up to 365 more days. We delete them when that period ends, up to 395 days after the request finishes, or sooner if we grant a deletion request. We retain them longer when required by law. Residual backup copies expire within [verified backup expiry period after deletion]. If we restore a backup, we apply prior deletions before making restored data available. Request and billing details, such as the request date, model, usage, and charges, follow the separate retention periods described in [request and billing data retention].
        </p>
      </section>
      <section>
        <h2>Your Model Training choice</h2>
        <p>
          Model Training is off by default. If you turn it on, we may use selected inputs and outputs from eligible requests you submit after enabling it to train, fine-tune, and evaluate TokenCraft models. We keep identifiable training copies for up to 395 days from the original request’s completion, subject to earlier deletion. You can turn Model Training off at any time in Settings → Privacy. We stop future training and evaluation use of your inputs and outputs and remove retained training copies through our deletion process. Your request history follows the same retention schedule. Contact us at [privacy request link or email] to request deletion, exercise other applicable privacy rights, or ask about content already used in training.
        </p>
      </section>
    </>
  );
}
