/** The initials-monogram gradient palette. Slugs are stored server-side
 * (`enums::MonogramGradient`); the actual color stops live here. */
export const MONOGRAM_GRADIENTS: { slug: string; label: string; css: string }[] = [
  { slug: "brand", label: "Brand", css: "linear-gradient(135deg, hsl(var(--brand)) 0%, hsl(var(--brand-2, var(--brand))) 100%)" },
  { slug: "ocean", label: "Ocean", css: "linear-gradient(135deg, #0ea5e9 0%, #2563eb 100%)" },
  { slug: "sunset", label: "Sunset", css: "linear-gradient(135deg, #f97316 0%, #db2777 100%)" },
  { slug: "forest", label: "Forest", css: "linear-gradient(135deg, #22c55e 0%, #0d9488 100%)" },
  { slug: "berry", label: "Berry", css: "linear-gradient(135deg, #8b5cf6 0%, #d946ef 100%)" },
  { slug: "gold", label: "Gold", css: "linear-gradient(135deg, #f59e0b 0%, #ca8a04 100%)" },
  { slug: "slate", label: "Slate", css: "linear-gradient(135deg, #64748b 0%, #334155 100%)" },
  { slug: "rose", label: "Rose", css: "linear-gradient(135deg, #fb7185 0%, #e11d48 100%)" },
];

/** CSS background for a stored slug (null/unknown → the default brand gradient). */
export function monogramGradientCss(slug: string | null | undefined): string {
  return (MONOGRAM_GRADIENTS.find((g) => g.slug === slug) ?? MONOGRAM_GRADIENTS[0]).css;
}
