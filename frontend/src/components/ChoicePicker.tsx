import { useEffect, useId, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { Check, ChevronDown } from "lucide-react";
import "./choice-picker.css";

type Choice = {
  value: string;
  label: string;
  detail?: ReactNode;
  trailing?: ReactNode;
  icon?: ReactNode;
  keywords?: readonly string[];
};

export function ChoicePicker({ label, options, value, onChange, compact = false, placeholder }: {
  label: string;
  options: readonly Choice[];
  value: string;
  onChange: (value: string) => void;
  compact?: boolean;
  placeholder?: string;
}) {
  const id = useId();
  const labelId = id + "-label";
  const listId = id + "-list";
  const valueId = id + "-value";
  const picker = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const search = useRef({ text: "", at: 0 });
  const [active, setActive] = useState<number | null>(null);
  const open = active !== null;
  const valueIndex = options.findIndex((option) => option.value === value);
  const selectedIndex = placeholder ? valueIndex : Math.max(0, valueIndex);
  const selected = options[selectedIndex];
  const firstActive = Math.max(0, selectedIndex);

  useEffect(() => {
    if (!open) return;
    function dismiss(event: PointerEvent) {
      if (event.target instanceof Node && !picker.current?.contains(event.target)) setActive(null);
    }
    document.addEventListener("pointerdown", dismiss, true);
    return () => document.removeEventListener("pointerdown", dismiss, true);
  }, [open]);

  useEffect(() => {
    if (active === null || !list.current) return;
    const menu = list.current;
    const option = menu.children[active] as HTMLElement | undefined;
    if (!option) return;
    const top = option.offsetTop;
    const bottom = top + option.offsetHeight;
    if (top < menu.scrollTop) menu.scrollTop = top;
    else if (bottom > menu.scrollTop + menu.clientHeight) menu.scrollTop = bottom - menu.clientHeight;
  }, [active]);

  function close() {
    search.current = { text: "", at: 0 };
    setActive(null);
  }

  function choose(index: number) {
    close();
    const option = options[index];
    if (option) onChange(option.value);
  }

  function handleKey(event: KeyboardEvent<HTMLButtonElement>) {
    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp": {
        event.preventDefault();
        const offset = event.key === "ArrowDown" ? 1 : -1;
        setActive(active === null ? firstActive : Math.max(0, Math.min(active + offset, options.length - 1)));
        return;
      }
      case "Home":
      case "End":
        event.preventDefault();
        setActive(event.key === "Home" ? 0 : options.length - 1);
        return;
      case "Enter":
      case " ":
        event.preventDefault();
        if (event.repeat) return;
        if (active === null) setActive(firstActive);
        else choose(active);
        return;
      case "Escape":
        if (!open) return;
        event.preventDefault();
        event.stopPropagation();
        close();
        return;
      case "Tab":
        close();
        return;
    }

    if (event.key.length !== 1 || event.ctrlKey || event.metaKey || event.altKey) return;
    const now = performance.now();
    const letter = event.key.toLowerCase();
    const previous = now - search.current.at < 650 ? search.current.text : "";
    const query = previous + letter;
    const match = options.findIndex((option) =>
      [option.label, option.value, ...(option.keywords ?? [])].some((term) => term.toLowerCase().startsWith(query)));
    search.current = { text: query, at: now };
    if (match === -1) return;
    event.preventDefault();
    setActive(match);
  }

  if (!selected && !placeholder) return null;
  return (
    <div ref={picker} className={`relative flex min-w-0 flex-col gap-1.5${compact ? " choice-picker-compact" : ""}`}>
      <span className={compact ? "sr-only" : "block text-sm font-medium"} id={labelId}>{label}</span>
      <button type="button" role="combobox"
        aria-labelledby={`${labelId} ${valueId}`} aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-activedescendant={open ? `${listId}-${active}` : undefined}
        className="choice-picker-trigger" disabled={!options.length}
        onClick={() => { if (open) close(); else setActive(firstActive); }}
        onKeyDown={handleKey} onBlur={close}>
        {selected ? <ChoiceContent option={selected} id={valueId} /> : (
          <span id={valueId} className="min-w-0 flex-1 truncate text-sm text-muted">{placeholder}</span>
        )}
        <ChevronDown className="choice-picker-chevron h-4 w-4 shrink-0 text-muted" aria-hidden />
      </button>
      {open && (
        <div id={listId} role="listbox" aria-labelledby={labelId} ref={list} className="choice-picker-list">
          {options.map((option, index) => (
            <div key={option.value} id={`${listId}-${index}`} role="option" aria-selected={index === selectedIndex}
              data-active={index === active} className="choice-picker-option"
              onMouseDown={(event) => event.preventDefault()} onClick={() => choose(index)}
              onPointerMove={(event) => { if (event.pointerType === "mouse") setActive(index); }}>
              <ChoiceContent option={option} wrapLabel={compact} />
              {index === selectedIndex && <Check className="h-4 w-4 shrink-0 text-brand" aria-hidden />}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ChoiceContent({ option, id, wrapLabel = false }: { option: Choice; id?: string; wrapLabel?: boolean }) {
  return (
    <>
      {option.icon}
      <span id={id} className="flex min-w-0 flex-1 items-center gap-2">
        <span className="min-w-0 flex-1">
          <span className={`block text-sm font-semibold leading-5 ${wrapLabel ? "break-words" : "truncate"}`}>{option.label}</span>
          {option.detail && <span className="block truncate text-xs leading-4 text-muted">{option.detail}</span>}
        </span>
        {option.trailing}
      </span>
    </>
  );
}
