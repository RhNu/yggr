import type { InputHTMLAttributes } from "react";

interface Props extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export default function Input({ label, className = "", id, ...props }: Props) {
  const inputId = id || (label ? label.toLowerCase().replace(/\s+/g, "-") : undefined);

  return (
    <div className="mb-4">
      {label && (
        <label htmlFor={inputId} className="mb-1.5 block text-xs text-neutral-500">
          {label}
        </label>
      )}
      <input
        id={inputId}
        className={`w-full rounded-md border border-white/10 bg-white/5 px-3 py-2 text-sm text-neutral-100 transition-colors outline-none placeholder:text-neutral-600 focus:border-white/25 ${className}`}
        {...props}
      />
    </div>
  );
}
