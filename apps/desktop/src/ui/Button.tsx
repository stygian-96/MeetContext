import type { ButtonHTMLAttributes } from "react";

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "record";
  size?: "small";
  pill?: boolean;
};

export function Button({ variant, size, pill, className = "", ...rest }: Props) {
  const cls = ["ui-button", variant && `ui-button-${variant}`, size && `ui-button-${size}`, pill && "ui-button-pill", className];
  return <button className={cls.filter(Boolean).join(" ")} {...rest} />;
}
