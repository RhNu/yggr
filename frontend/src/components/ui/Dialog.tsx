import type { ReactNode } from "react";
import {
  Dialog as HeadlessDialog,
  DialogBackdrop,
  DialogPanel,
  DialogTitle,
} from "@headlessui/react";

interface Props {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}

export default function Dialog({
  open,
  title,
  onClose,
  children,
  footer,
}: Props) {
  return (
    <HeadlessDialog
      open={open}
      onClose={onClose}
      transition
      className="relative z-50 transition duration-200 ease-out data-closed:opacity-0"
    >
      <DialogBackdrop className="fixed inset-0 bg-black/60 backdrop-blur-sm" />
      <div className="fixed inset-0 flex w-screen items-center justify-center p-4">
        <DialogPanel
          transition
          className="w-full max-w-lg rounded-xl border border-white/10 bg-neutral-900/80 backdrop-blur-md p-6 shadow-2xl transition duration-200 ease-out data-closed:scale-95 data-closed:opacity-0"
        >
          <div className="mb-4 flex items-center justify-between">
            <DialogTitle className="text-base font-semibold text-neutral-100">
              {title}
            </DialogTitle>
          </div>
          <div className="mb-4">{children}</div>
          {footer && (
            <div className="flex justify-end gap-2">{footer}</div>
          )}
        </DialogPanel>
      </div>
    </HeadlessDialog>
  );
}
