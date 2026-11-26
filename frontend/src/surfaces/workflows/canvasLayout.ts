export type Position = { x: number; y: number };

export const CARD_WIDTH = 188;
export const CARD_ROW = 246;
export const BOARD_LIMIT = 2600;

function gridPosition(index: number): Position {
  const row = Math.floor(index / 3);
  const column = index % 3;
  return { x: 20 + (row % 2 ? 2 - column : column) * 226, y: 20 + row * CARD_ROW };
}

export function placeCards<Node extends { id: string }>(nodes: Node[], custom: Record<string, Position>) {
  const request = gridPosition(0);
  const occupied = [request, ...nodes.flatMap(({ id }) => custom[id] ?? [])];
  let nextSlot = 1;

  function nextPosition(): Position {
    let position = gridPosition(nextSlot++);
    while (occupied.some(({ x, y }) => Math.abs(position.x - x) < CARD_WIDTH + 8
      && Math.abs(position.y - y) < CARD_ROW - 18)) {
      position = gridPosition(nextSlot++);
    }
    occupied.push(position);
    return position;
  }

  return [
    { id: ":inputs", node: undefined, position: request },
    ...nodes.map((node) => ({ id: node.id, node, position: custom[node.id] ?? nextPosition() })),
    { id: ":outputs", node: undefined, position: nextPosition() },
  ];
}

function validPosition(position: unknown): position is Position {
  if (!position || typeof position !== "object" || !("x" in position) || !("y" in position)) return false;
  return [position.x, position.y].every((value) => typeof value === "number" && value >= 12 && value <= BOARD_LIMIT);
}

export function loadPositions(key: string): Record<string, Position> {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(key) ?? "{}");
    if (!value || typeof value !== "object") return {};
    return Object.fromEntries(Object.entries(value).filter(([id, position]) => /^[a-z][a-z0-9_]{0,47}$/.test(id)
      && validPosition(position)));
  } catch {
    return {};
  }
}
