import { useEffect, useState } from "react";
import Dialog from "./ui/Dialog";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { createPlayer } from "../api";

interface Props {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}

export default function CreatePlayerDialog({ open, onClose, onCreated }: Props) {
  const [name, setName] = useState("");
  const [model, setModel] = useState("classic");
  const { busy, error, run } = useAsyncAction();

  useEffect(() => {
    if (open) {
      setName("");
      setModel("classic");
    }
  }, [open]);

  const handleSubmit = async () => {
    const result = await run(() => createPlayer(name.trim(), model));
    if (result !== null) {
      onClose();
      onCreated();
    }
  };

  return (
    <Dialog
      open={open}
      title="Add Character"
      onClose={onClose}
      footer={
        <>
          <button
            className="secondary btn-sm"
            onClick={onClose}
            disabled={busy}
          >
            Cancel
          </button>
          <button
            className="btn-sm"
            onClick={handleSubmit}
            disabled={busy || !name.trim()}
          >
            {busy ? "..." : "Add"}
          </button>
        </>
      }
    >
      <div className="form-group">
        <label>Player Name</label>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="3-16 chars, a-z A-Z 0-9 _"
          maxLength={16}
          autoFocus
        />
      </div>
      <div className="form-group">
        <label>Model</label>
        <select value={model} onChange={(e) => setModel(e.target.value)}>
          <option value="classic">Classic</option>
          <option value="slim">Slim</option>
        </select>
      </div>
      {error && <div className="error-msg">{error}</div>}
    </Dialog>
  );
}
