/** Tiny inline usage sparkline (SVG). Colored green/red by net direction. */
export function Sparkline({ data, width = 88, height = 26, className }: { data: number[]; width?: number; height?: number; className?: string }) {
  if (!data.length) return null;
  const min = Math.min(...data);
  const max = Math.max(...data);
  const span = max - min || 1;
  const pts = data
    .map((v, i) => `${(i / (data.length - 1)) * width},${height - ((v - min) / span) * (height - 4) - 2}`)
    .join(" ");
  const up = data[data.length - 1] >= data[0];
  return (
    <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} className={className} preserveAspectRatio="none">
      <polyline
        points={pts}
        fill="none"
        stroke={up ? "hsl(var(--brand))" : "hsl(350 80% 62%)"}
        strokeWidth="1.75"
        strokeLinejoin="round"
        strokeLinecap="round"
      />
    </svg>
  );
}
