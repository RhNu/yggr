import type { SelectHTMLAttributes } from "react";

interface Props extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
}

export default function Select({ label, className = "", id, children, ...props }: Props) {
  const selectId = id || (label ? label.toLowerCase().replace(/\s+/g, "-") : undefined);

  return (
    <div className="mb-4">
      {label && (
        <label htmlFor={selectId} className="mb-1.5 block text-xs text-neutral-500">
          {label}
        </label>
      )}
      <select
        id={selectId}
        className={`w-full rounded-md border border-white/10 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 transition-colors outline-none focus:border-white/25 ${className}`}
        {...props}
      >
        {children}
      </select>
    </div>
  );
}
