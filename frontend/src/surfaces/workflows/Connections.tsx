import { useLayoutEffect, useRef, useState } from "react";
import type { Position } from "./canvasLayout";
import type { Graph } from "./types";
import { inputPort, outputPort, sourcePort, wireSources } from "./wiring";

type Edge = { from: string; to: string; selected: boolean };

export function Connections({ graph, selected, layout }: {
  graph: Graph; selected?: string; layout: Record<string, Position>;
}) {
  const drawing = useRef<SVGSVGElement>(null);
  const [paths, setPaths] = useState<{ id: string; d: string; selected: boolean }[]>([]);
  useLayoutEffect(() => {
    const canvas = drawing.current?.parentElement;
    if (!canvas) return;
    const edges: Edge[] = graph.nodes.flatMap((step) =>
      Object.entries(step.settings).flatMap(([field, binding]) =>
        [...new Set(wireSources(binding).map(sourcePort))].map((from) => ({
          from, to: inputPort(step.id, field), selected: step.id === selected || from === `step:${selected}`,
        }))));
    for (const [name, id] of Object.entries(graph.outputs)) {
      edges.push({ from: `step:${id}`, to: outputPort(name), selected: id === selected });
    }

    function draw() {
      if (!canvas) return;
      const origin = canvas.getBoundingClientRect();
      const ports = new Map([...canvas.querySelectorAll<HTMLElement>("[data-wire-port]")].map((element) => {
        const dot = element.querySelector("i")?.getBoundingClientRect();
        const area = element.closest(".wf-node-ports")?.getBoundingClientRect();
        const visible = dot?.width && (!area || (dot.top >= area.top && dot.bottom <= area.bottom));
        return [element.dataset.wirePort, visible ? dot : undefined];
      }));
      setPaths(edges.flatMap(({ from, to, selected }) => {
        const start = ports.get(from);
        const end = ports.get(to);
        if (!start || !end) return [];
        const x1 = start.left + start.width / 2 - origin.left;
        const x2 = end.left + end.width / 2 - origin.left;
        const y1 = start.top + start.height / 2 - origin.top;
        const y2 = end.top + end.height / 2 - origin.top;
        const reach = Math.max(26, Math.abs(x2 - x1) * .44);
        return [{ id: `${from}-${to}`, d: `M${x1} ${y1} C${x1 + reach} ${y1} ${x2 - reach} ${y2} ${x2} ${y2}`, selected }];
      }));
    }

    draw();
    const observer = new ResizeObserver(draw);
    observer.observe(canvas);
    canvas.querySelectorAll(".wf-canvas-card").forEach((card) => observer.observe(card));
    canvas.addEventListener("scroll", draw, true);
    return () => {
      observer.disconnect();
      canvas.removeEventListener("scroll", draw, true);
    };
  }, [graph, selected, layout]);

  return <svg ref={drawing} className="wf-connections" aria-hidden="true">{paths.map(({ id, d, selected }) =>
    <path d={d} key={id} className={selected ? "wf-path-selected" : undefined} />)}</svg>;
}
