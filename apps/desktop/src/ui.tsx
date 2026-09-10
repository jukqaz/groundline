import { type ComponentProps, type ReactNode } from "react";
import { Button as RadixButton, TextField, Select } from "@radix-ui/themes";
import { Check, Info, X } from "lucide-react";

export function Button({
  className = "",
  type,
  ...props
}: ComponentProps<typeof RadixButton>) {
  const primary = className.split(" ").includes("primary");
  const quiet = /nav-item|theme-|text-button|notice-dismiss/.test(className);
  return (
    <RadixButton
      size="2"
      variant={primary ? "solid" : quiet ? "ghost" : "surface"}
      color="gray"
      highContrast={primary}
      type={type ?? "submit"}
      className={className}
      {...props}
    />
  );
}

export function Input(props: ComponentProps<typeof TextField.Root>) {
  return <TextField.Root size="2" variant="surface" {...props} />;
}

export function Choice({
  label,
  value,
  onValueChange,
  disabled,
  options,
  id,
}: {
  label: string;
  value: string;
  onValueChange: (value: string) => void;
  disabled?: boolean;
  options: readonly (readonly [string, string])[];
  id?: string;
}) {
  return (
    <Select.Root
      value={value}
      onValueChange={onValueChange}
      disabled={disabled}
      size="2"
    >
      <Select.Trigger id={id} aria-label={label} className="choice-control" />
      <Select.Content position="popper" variant="soft">
        {options.map(([value, label]) => (
          <Select.Item key={value} value={value}>
            {label}
          </Select.Item>
        ))}
      </Select.Content>
    </Select.Root>
  );
}
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
        <Button
          className="notice-dismiss"
          aria-label="알림 닫기"
          onClick={onDismiss}
        >
          <X size={16} />
        </Button>
      )}
    </div>
  );
}
