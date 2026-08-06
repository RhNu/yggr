import Dialog from "./Dialog";

interface Props {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  busy?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  busy = false,
  onConfirm,
  onCancel,
}: Props) {
  return (
    <Dialog
      open={open}
      title={title}
      onClose={onCancel}
      footer={
        <>
          <button
            className="secondary btn-sm"
            onClick={onCancel}
            disabled={busy}
          >
            {cancelLabel}
          </button>
          <button className="danger btn-sm" onClick={onConfirm} disabled={busy}>
            {busy ? "..." : confirmLabel}
          </button>
        </>
      }
    >
      <p className="confirm-message">{message}</p>
    </Dialog>
  );
}
