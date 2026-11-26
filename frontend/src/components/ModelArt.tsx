import { useState, type ReactNode } from "react";
import type { Model } from "@/lib/catalog";
import { cn } from "@/lib/cn";

const creatorMarks = new Set(["anthropic", "blackforestlabs", "bytedance", "google", "meta", "openai", "tencent", "worldlabs"]);

export type ModelCreator = Pick<Model, "provider" | "providerSlug">;

export function ModelIcon({ model, className, fallback }: {
  model?: ModelCreator;
  className?: string;
  fallback?: ReactNode;
}) {
  const [failed, setFailed] = useState<string>();
  const slug = model?.providerSlug;
  return (
    <span aria-hidden="true" className={cn("grid h-9 w-9 shrink-0 place-items-center overflow-hidden rounded-md border border-border bg-white", className)}>
      {slug && creatorMarks.has(slug) && failed !== slug ? (
        <img src={`/brand/model-creators/${slug}.png`} alt="" width={32} height={32} decoding="async"
          onError={() => setFailed(slug)} className="h-[76%] w-[76%] object-contain" />
      ) : model ? (
        <span className="font-mono text-xs font-bold text-neutral-700">{model.provider.split(/\s+/).map((word) => word[0]).join("").slice(0, 2).toUpperCase()}</span>
      ) : fallback}
    </span>
  );
}
