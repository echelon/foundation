import { useState } from "react";
import { AVATAR_URL, type ApiUser } from "@/lib/api";
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
 * The user's avatar. When the gravatar-job has confirmed a gravatar
 * (`avatar_source === "gravatar"`), loads the image through the gateway proxy
 * (cookie-authed, hence `use-credentials`); otherwise — or if the image fails —
 * renders the letter monogram.
 */
export function UserAvatar({ user, className }: { user: ApiUser; className?: string }) {
  const [failed, setFailed] = useState(false);
  const showImage = user.avatar_source === "gravatar" && !failed;

  if (showImage) {
    return (
      <img
        src={AVATAR_URL}
        crossOrigin="use-credentials"
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
