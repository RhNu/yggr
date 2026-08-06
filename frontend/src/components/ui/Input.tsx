import type { InputHTMLAttributes } from "react";

interface Props extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export default function Input({ label, className = "", id, ...props }: Props) {
  const inputId = id || (label ? label.toLowerCase().replace(/\s+/g, "-") : undefined);

  return (
    <div className="mb-4">
      {label && (
        <label
          htmlFor={inputId}
          className="block mb-1.5 text-xs text-neutral-500"
        >
          {label}
        </label>
      )}
      <input
        id={inputId}
        className={`w-full rounded-md bg-white/5 border border-white/10 px-3 py-2 text-sm text-neutral-100 outline-none transition-colors focus:border-white/25 placeholder:text-neutral-600 ${className}`}
        {...props}
      />
    </div>
  );
}
