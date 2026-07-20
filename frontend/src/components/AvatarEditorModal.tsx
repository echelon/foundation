import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { X } from "lucide-react";
import { Button, Modal, Slider } from "@/components/ui";
import { api, ApiError, type AvatarCrop } from "@/lib/api";

/** Square editing viewport size (CSS pixels). */
const VIEWPORT = 280;
const MAX_ZOOM = 4;

/**
 * Crop/zoom editor for avatar uploads. Drag to pan, slider to zoom; the square crop rect is
 * tracked in SOURCE-image pixels and sent as `crop_x/crop_y/crop_size`, so what you see in the
 * viewport is exactly what the server bakes into the 512×512 avatar.
 */
export function AvatarEditorModal({
  file,
  onClose,
  onSaved,
}: {
  file: File;
  onClose: () => void;
  onSaved: () => Promise<void> | void;
}) {
  // Create the blob URL inside the effect (NOT useMemo): under StrictMode's double-invoked
  // effects, a memoized URL gets revoked by the first cleanup and the remount would reuse the
  // dead URL — the image would never load and the editor would appear broken.
  const [objectUrl, setObjectUrl] = useState<string | null>(null);
  useEffect(() => {
    const url = URL.createObjectURL(file);
    setObjectUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [file]);

  // Source-image dimensions (known once the <img> loads).
  const [dims, setDims] = useState<{ w: number; h: number } | null>(null);
  const [zoom, setZoom] = useState(1);
  // Crop origin (top-left) in source pixels.
  const [origin, setOrigin] = useState({ x: 0, y: 0 });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const minDim = dims ? Math.min(dims.w, dims.h) : 0;
  const cropSize = dims ? Math.max(1, Math.floor(minDim / zoom)) : 1;
  const displayScale = VIEWPORT / cropSize;

  const clampOrigin = (x: number, y: number, size: number) => {
    if (!dims) return { x: 0, y: 0 };
    return {
      x: Math.min(Math.max(0, x), dims.w - size),
      y: Math.min(Math.max(0, y), dims.h - size),
    };
  };

  const onImageLoad = (w: number, h: number) => {
    setDims({ w, h });
    const size = Math.min(w, h);
    setOrigin({ x: Math.floor((w - size) / 2), y: Math.floor((h - size) / 2) });
  };

  // Zoom around the current crop center so the subject stays put.
  const onZoom = (z: number) => {
    if (!dims) return;
    const newSize = Math.max(1, Math.floor(minDim / z));
    const cx = origin.x + cropSize / 2;
    const cy = origin.y + cropSize / 2;
    setZoom(z);
    setOrigin(clampOrigin(Math.round(cx - newSize / 2), Math.round(cy - newSize / 2), newSize));
  };

  // Drag to pan.
  const drag = useRef<{ startX: number; startY: number; originX: number; originY: number } | null>(null);
  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { startX: e.clientX, startY: e.clientY, originX: origin.x, originY: origin.y };
  };
  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (!drag.current || !dims) return;
    const dx = (e.clientX - drag.current.startX) / displayScale;
    const dy = (e.clientY - drag.current.startY) / displayScale;
    setOrigin(clampOrigin(Math.round(drag.current.originX - dx), Math.round(drag.current.originY - dy), cropSize));
  };
  const onPointerUp = () => {
    drag.current = null;
  };

  const save = async () => {
    if (!dims) return;
    setBusy(true);
    setError(null);
    const crop: AvatarCrop = { ...clampOrigin(origin.x, origin.y, cropSize), size: cropSize };
    try {
      await api.uploadAvatar(file, crop);
      await onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Upload failed — try again.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal open onClose={onClose} className="max-w-md">
      <div className="flex items-start justify-between">
        <div>
          <h2 className="font-display text-xl font-bold tracking-tight">Adjust your photo</h2>
          <p className="mt-1 text-sm text-muted">Drag to reposition · slide to zoom.</p>
        </div>
        <button onClick={onClose} className="rounded-md p-1 text-muted transition hover:text-fg" aria-label="Close">
          <X className="h-5 w-5" />
        </button>
      </div>

      <div className="mt-4 flex flex-col items-center gap-4">
        {/* Viewport: the visible square IS the crop. */}
        <div
          className="relative touch-none select-none overflow-hidden rounded-xl border border-border bg-surface2"
          style={{ width: VIEWPORT, height: VIEWPORT, cursor: dims ? "grab" : "default" }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
        >
          {objectUrl && (
          <img
            src={objectUrl}
            alt=""
            draggable={false}
            onLoad={(e) => onImageLoad(e.currentTarget.naturalWidth, e.currentTarget.naturalHeight)}
            className="pointer-events-none absolute max-w-none"
            style={
              dims
                ? {
                    width: dims.w * displayScale,
                    height: dims.h * displayScale,
                    left: -origin.x * displayScale,
                    top: -origin.y * displayScale,
                  }
                : { opacity: 0 }
            }
          />
          )}
          {/* Circular guide — how the avatar will actually render. */}
          <div className="pointer-events-none absolute inset-0 rounded-full border-2 border-white/60 shadow-[0_0_0_9999px_rgba(0,0,0,0.25)]" />
        </div>

        <div className="w-full max-w-[280px]">
          <Slider value={zoom} min={1} max={MAX_ZOOM} step={0.05} onChange={onZoom} />
          <div className="mt-1 flex justify-between text-xs text-muted">
            <span>Zoom</span>
            <span>{zoom.toFixed(2)}×</span>
          </div>
        </div>
      </div>

      {error && <p className="mt-3 rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-500">{error}</p>}

      <div className="mt-4 flex justify-end gap-2">
        <Button variant="outline" onClick={onClose} disabled={busy}>
          Cancel
        </Button>
        <Button onClick={save} disabled={busy || !dims}>
          {busy ? "Uploading…" : "Save photo"}
        </Button>
      </div>
    </Modal>
  );
}
