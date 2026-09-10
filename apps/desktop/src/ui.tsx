import { type ReactNode } from "react";
import { Check, Info, X } from "lucide-react";
export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
      {hint && <small>{hint}</small>}
    </label>
  );
}
export function Notice({
  children,
  kind = "info",
  onDismiss,
}: {
  children: ReactNode;
  kind?: "info" | "success" | "error";
  onDismiss?: () => void;
}) {
  return (
    <div
      className={"notice " + kind}
      role={kind === "error" ? "alert" : "status"}
    >
      {kind === "success" ? <Check size={17} /> : <Info size={17} />}
      <div>{children}</div>
      {onDismiss && (
        <button
          className="notice-dismiss"
          aria-label="알림 닫기"
          onClick={onDismiss}
        >
          <X size={16} />
        </button>
      )}
    </div>
  );
}
