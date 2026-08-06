import type { SelectHTMLAttributes } from "react";

interface Props extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
}

export default function Select({ label, className = "", id, children, ...props }: Props) {
  const selectId = id || (label ? label.toLowerCase().replace(/\s+/g, "-") : undefined);

  return (
    <div className="mb-4">
      {label && (
        <label
          htmlFor={selectId}
          className="block mb-1.5 text-xs text-neutral-500"
        >
          {label}
        </label>
      )}
      <select
        id={selectId}
        className={`w-full rounded-md bg-neutral-900 border border-white/10 px-3 py-2 text-sm text-neutral-100 outline-none transition-colors focus:border-white/25 ${className}`}
        {...props}
      >
        {children}
      </select>
    </div>
  );
}
