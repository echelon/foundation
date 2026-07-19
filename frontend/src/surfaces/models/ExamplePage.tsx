import { useEffect } from "react";
import { Link, Navigate, useParams } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import { Container, Badge } from "@/components/ui";
import { ExampleDetailBody } from "./Gallery";
import { exampleById } from "@/lib/gallery";
import { MODEL_BY_SLUG } from "@/lib/catalog";

/**
 * Standalone, deep-linkable example page — the SEO unit
 * (`design/subsystems/prompt_gallery.md` §5). In production this is prerendered
 * with JSON-LD (ImageObject/VideoObject) + OG tags; here we set the title.
 */
export default function ExamplePage() {
  const { slug, exampleId } = useParams();
  const example = slug && exampleId ? exampleById(slug, exampleId) : undefined;
  const model = slug ? MODEL_BY_SLUG[slug] : undefined;

  useEffect(() => {
    if (example && model) {
      document.title = `${example.title} — ${model.name} prompt · Foundation`;
      return () => {
        document.title = "Foundation";
      };
    }
  }, [example, model]);

  if (!example || !model) return <Navigate to={slug ? `/models/${slug}` : "/models"} replace />;

  return (
    <Container className="max-w-5xl py-10">
      <Link to={`/models/${slug}`} className="mb-6 inline-flex items-center gap-1.5 text-sm text-muted hover:text-fg">
        <ArrowLeft className="h-4 w-4" /> {model.name} gallery
      </Link>

      <div className="mb-2 flex items-center gap-2 text-sm text-muted">
        <Link to={`/models/${slug}`} className="hover:text-fg">
          {model.name}
        </Link>
        <span>/</span>
        <span>Gallery</span>
        {example.isVideo && <Badge tone="brand">video</Badge>}
      </div>
      <h1 className="mb-6 font-display text-3xl font-extrabold tracking-tight">{example.title}</h1>

      <div className="grid gap-6 overflow-hidden rounded-2xl border border-border bg-surface lg:grid-cols-[1.2fr_1fr]">
        <div className="bg-surface2/40 p-4">
          <div className="overflow-hidden rounded-xl border border-border">
            {example.isVideo ? (
              <video src={example.assetUrl} poster={example.posterUrl} controls loop muted playsInline className="w-full" />
            ) : (
              <img src={example.assetUrl} alt={example.title} className="w-full" />
            )}
          </div>
          {example.sourceAssetUrl && (
            <div className="mt-3">
              <p className="mb-1 text-xs text-muted">Input image (image-to-video)</p>
              <img src={example.sourceAssetUrl} alt="source" className="h-24 rounded-lg border border-border object-cover" />
            </div>
          )}
        </div>
        <ExampleDetailBody example={example} />
      </div>
    </Container>
  );
}
