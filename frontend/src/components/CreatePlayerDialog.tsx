import { useEffect, useState } from "react";
import Dialog from "./ui/Dialog";
import Button from "./ui/Button";
import Input from "./ui/Input";
import Select from "./ui/Select";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { createPlayer } from "../api";

interface Props {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}

export default function CreatePlayerDialog({
  open,
  onClose,
  onCreated,
}: Props) {
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
          <Button
            variant="secondary"
            size="sm"
            onClick={onClose}
            disabled={busy}
          >
            Cancel
          </Button>
          <Button
            size="sm"
            onClick={handleSubmit}
            disabled={busy || !name.trim()}
          >
            {busy ? "..." : "Add"}
          </Button>
        </>
      }
    >
      <Input
        label="Player Name"
        type="text"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="3-16 chars, a-z A-Z 0-9 _"
        maxLength={16}
        autoFocus
      />
      <Select
        label="Model"
        value={model}
        onChange={(e) => setModel(e.target.value)}
      >
        <option value="classic">Classic</option>
        <option value="slim">Slim</option>
      </Select>
      {error && <p className="text-sm text-red-400">{error}</p>}
    </Dialog>
  );
}
