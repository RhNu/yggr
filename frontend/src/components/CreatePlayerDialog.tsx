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

const UUID_RE =
  /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;
const UUID_SIMPLE_RE = /^[0-9a-fA-F]{32}$/;

function isValidUuid(value: string): boolean {
  return UUID_RE.test(value) || UUID_SIMPLE_RE.test(value);
}

export default function CreatePlayerDialog({ open, onClose }: Props) {
  const [name, setName] = useState("");
  const [model, setModel] = useState("classic");
  const [uuid, setUuid] = useState("");
  const createPlayer = useCreatePlayer();

  useEffect(() => {
    if (open) {
      setName("");
      setModel("classic");
      setUuid("");
    }
  }, [open]);

  const trimmedUuid = uuid.trim();
  const uuidError =
    trimmedUuid !== "" && !isValidUuid(trimmedUuid)
      ? "Invalid UUID format, expected 8-4-4-4-12 or 32 hex digits"
      : null;

  const handleSubmit = () => {
    createPlayer.mutate(
      { name: name.trim(), skinModel: model, uuid: trimmedUuid || undefined },
      { onSuccess: () => onClose() },
    );
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
            disabled={createPlayer.isPending}
          >
            Cancel
          </Button>
          <Button
            size="sm"
            onClick={handleSubmit}
            disabled={
              createPlayer.isPending || !name.trim() || Boolean(uuidError)
            }
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
      <Select
        label="Model"
        value={model}
        onChange={(e) => setModel(e.target.value)}
      >
        <option value="classic">Classic</option>
        <option value="slim">Slim</option>
      </Select>
      <Input
        label="UUID (optional)"
        type="text"
        value={uuid}
        onChange={(e) => setUuid(e.target.value)}
        placeholder="8-4-4-4-12 or 32 hex, leave empty for auto"
        maxLength={36}
      />
      {uuidError && <p className="text-sm text-red-400">{uuidError}</p>}
      {createPlayer.error && (
        <p className="text-sm text-red-400">{createPlayer.error.message}</p>
      )}
    </Dialog>
  );
}
