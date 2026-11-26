import { useState } from "react";
import { Clapperboard, Image, Plus, Search, TextCursorInput, X, type LucideIcon } from "lucide-react";
import { MODEL_BY_SLUG } from "@/lib/catalog";
import type { Capabilities, PrimitiveKind } from "./types";
import { COMPONENT_TRANSFER } from "./wiring";

export type ComponentChoice = { kind: PrimitiveKind; model?: string };
export const primitiveIcons = { text: TextCursorInput, image: Image, video: Clapperboard };

const componentTasks: Record<PrimitiveKind, string> = {
  text: "Copy, prompts, summaries",
  image: "Artwork, photos, variations",
  video: "Clips, motion, animation",
};

export function modelName(id: string) {
  return MODEL_BY_SLUG[id]?.name ?? (id || "Choose a model");
}

export function ComponentLibrary({ capabilities, canAdd, onAdd, onClose }: {
  capabilities: Capabilities; canAdd: boolean; onAdd: (choice: ComponentChoice) => void; onClose: () => void;
}) {
  const [search, setSearch] = useState("");
  const query = search.trim().toLowerCase();
  const matches = (value: string) => query.split(/\s+/).every((term) => value.toLowerCase().includes(term));
  const components = capabilities.primitives.filter((primitive) =>
    matches(`${primitive.label} ${primitive.kind} ${componentTasks[primitive.kind]}`));
  const models = query ? capabilities.models.filter((model) =>
    matches(`${modelName(model.id)} ${model.id} ${model.providers.join(" ")} ${model.kind}`)).slice(0, 24) : [];

  function item(choice: ComponentChoice, title: string, detail: string, Icon: LucideIcon) {
    return <button className="wf-component" key={choice.model ?? choice.kind} type="button" disabled={!canAdd}
      draggable={canAdd} onClick={() => onAdd(choice)} onDragStart={(event) => {
        event.dataTransfer.setData(COMPONENT_TRANSFER, JSON.stringify(choice));
        event.dataTransfer.effectAllowed = "copy";
      }}>
      <span className={`wf-node-icon wf-node-${choice.kind}`}><Icon size={15} /></span>
      <span><strong>{title}</strong><small>{detail}</small></span>
      <Plus size={14} aria-hidden="true" />
    </button>;
  }

  return <section className="wf-library-options" aria-label="Component library" onKeyDown={(event) => {
    if (event.key !== "Escape") return;
    event.stopPropagation();
    onClose();
  }}>
    <div className="wf-panel-heading"><h3>Component library</h3>
      <button className="wf-icon-button" type="button" aria-label="Close component library" onClick={onClose}><X size={16} /></button>
    </div>
    <label className="wf-library-search"><Search size={15} aria-hidden="true" />
      <input type="search" aria-label="Search components and models" placeholder="Search components and models"
        value={search} autoFocus onChange={(event) => setSearch(event.target.value)} />
    </label>
    <div className="wf-library-results">
      {components.map((primitive) => item({ kind: primitive.kind }, primitive.label,
        componentTasks[primitive.kind], primitiveIcons[primitive.kind]))}
      {models.length > 0 && <h4>Models</h4>}
      {models.map((model) => item({ kind: model.kind, model: model.id }, modelName(model.id),
        model.providers.join(" · "), primitiveIcons[model.kind]))}
      {components.length + models.length === 0 && <p className="wf-muted" role="status">No matching components or models.</p>}
    </div>
    <small>{canAdd ? "Drag onto the canvas or select to add." : "This workflow has reached its 20-step limit."}</small>
  </section>;
}
