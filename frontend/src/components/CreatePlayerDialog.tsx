import { useEffect, useState } from "react";

import Button from "@/components/ui/Button";
import Dialog from "@/components/ui/Dialog";
import Input from "@/components/ui/Input";
import Select from "@/components/ui/Select";
import { useCreatePlayer } from "@/queries";

interface Props {
  open: boolean;
  onClose: () => void;
}

export default function CreatePlayerDialog({ open, onClose }: Props) {
  const [name, setName] = useState("");
  const [model, setModel] = useState("classic");
  const createPlayer = useCreatePlayer();

  useEffect(() => {
    if (open) {
      setName("");
      setModel("classic");
    }
  }, [open]);

  const handleSubmit = () => {
    createPlayer.mutate({ name: name.trim(), skinModel: model }, { onSuccess: () => onClose() });
  };

  return (
    <Dialog
      open={open}
      title="Add Character"
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" size="sm" onClick={onClose} disabled={createPlayer.isPending}>
            Cancel
          </Button>
          <Button
            size="sm"
            onClick={handleSubmit}
            disabled={createPlayer.isPending || !name.trim()}
          >
            {createPlayer.isPending ? "..." : "Add"}
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
      <Select label="Model" value={model} onChange={(e) => setModel(e.target.value)}>
        <option value="classic">Classic</option>
        <option value="slim">Slim</option>
      </Select>
      {createPlayer.error && <p className="text-sm text-red-400">{createPlayer.error.message}</p>}
    </Dialog>
  );
}
