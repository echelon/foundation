import { useState } from "react";
import type { ApiUser } from "@/lib/api";
import { cn } from "@/lib/cn";

/** Monogram initials from the display name (fallback: email). */
export function userInitials(user: ApiUser): string {
  const name = user.display_name?.trim();
  if (name) {
    const parts = name.split(/\s+/);
    const first = parts[0]?.[0] ?? "";
    const last = parts.length > 1 ? (parts[parts.length - 1][0] ?? "") : "";
    return (first + last).toUpperCase() || "?";
  }
  return (user.email[0] ?? "?").toUpperCase();
}

/**
 * The user's avatar. When the API reports CDN links (custom upload or mirrored gravatar),
 * renders the image straight from the CDN; otherwise — or if the image fails to load —
 * renders the letter monogram.
 */
export function UserAvatar({ user, className }: { user: ApiUser; className?: string }) {
  const [failed, setFailed] = useState(false);
  const links = user.avatar?.links ?? null;

  if (links && !failed) {
    return (
      <img
        src={links.cdn_url}
        alt=""
        onError={() => setFailed(true)}
        className={cn("rounded-full object-cover", className)}
      />
    );
  }
  return (
    <span
      className={cn(
        "grid place-items-center rounded-full brand-gradient font-semibold text-white",
        className,
      )}
    >
      {userInitials(user)}
    </span>
  );
}
