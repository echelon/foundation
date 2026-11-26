import { AudioLines, Box, Clapperboard, Image, Terminal } from "lucide-react";
import { ChoicePicker } from "@/components/ChoicePicker";
import { ModelIcon, type ModelCreator } from "@/components/ModelArt";
import { MODEL_BY_SLUG, type Model } from "@/lib/catalog";
import { modelNames, type ModelChoice } from "@/surfaces/requests/modelOptions";

const modalityIcons = { text: Terminal, image: Image, video: Clapperboard, audio: AudioLines, mesh: Box };
const catalogAliases = new Map([["studio_seedance_20", "bytedance_seedance_2"]]);

const additionalCreators = new Map<string, ModelCreator>([
  ["studio_flux_image", { provider: "Black Forest Labs", providerSlug: "blackforestlabs" }],
]);

function catalogModel(choice: ModelChoice): Model | undefined {
  if (Object.hasOwn(MODEL_BY_SLUG, choice.model)) return MODEL_BY_SLUG[choice.model];
  const alias = catalogAliases.get(choice.model);
  return alias ? MODEL_BY_SLUG[alias] : undefined;
}

function modelCreator(choice: ModelChoice): ModelCreator | undefined {
  return catalogModel(choice) ?? additionalCreators.get(choice.model);
}

function displayName(choice: ModelChoice) {
  if (Object.hasOwn(modelNames, choice.model)) return modelNames[choice.model];
  return catalogModel(choice)?.name ?? choice.model;
}

export function ModelPicker({ choices, selected, onChange }: {
  choices: [string, ModelChoice][];
  selected: [string, ModelChoice];
  onChange: (id: string) => void;
}) {
  return (
    <ChoicePicker label="Model" value={selected[0]} onChange={onChange} options={choices.map(([value, model]) => {
      const creator = modelCreator(model);
      const Icon = modalityIcons[model.kind];
      const label = displayName(model);
      const showId = label !== model.model;
      return {
        value, label,
        keywords: [model.model, creator?.provider ?? ""],
        icon: <ModelIcon model={creator} fallback={<Icon className="h-4 w-4" />}
          className={creator ? undefined : "border-brand/15 bg-brand/10 text-brand"} />,
        detail: (creator || showId) && (
          <span title={model.model}>
            {creator?.provider}
            {showId && <span className="font-mono text-[11px]">{creator && " · "}{model.model}</span>}
          </span>
        ),
      };
    })} />
  );
}
