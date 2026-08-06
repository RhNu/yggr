import { useEffect, useState } from "react";

import { uploadTexture, textureUrl, type Player } from "@/api";
import SkinPreview from "@/components/SkinPreview";
import Button from "@/components/ui/Button";
import Dialog from "@/components/ui/Dialog";
import Input from "@/components/ui/Input";
import { useAsyncAction } from "@/hooks/useAsyncAction";

interface Props {
  open: boolean;
  player: Player;
  onClose: () => void;
  onChanged: () => void;
}

export default function UploadCapeDialog({ open, player, onClose, onChanged }: Props) {
  const [file, setFile] = useState<File | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const { busy, error, run } = useAsyncAction();

  useEffect(() => {
    if (open) {
      setFile(null);
      setPreviewUrl(null);
    }
  }, [open]);

  useEffect(() => {
    return () => {
      if (previewUrl) URL.revokeObjectURL(previewUrl);
    };
  }, [previewUrl]);

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (!f) return;
    if (previewUrl) URL.revokeObjectURL(previewUrl);
    const url = URL.createObjectURL(f);
    setFile(f);
    setPreviewUrl(url);
    e.target.value = "";
  };

  const handleConfirm = async () => {
    if (!file) return;
    const result = await run(() => uploadTexture(player.id, "cape", file));
    if (result !== null) {
      onClose();
      onChanged();
    }
  };

  const currentSkinUrl = player.skin_hash
    ? textureUrl(player.skin_hash)
    : `/service/textures/default/${player.skin_model}`;
  const currentCapeUrl = player.cape_hash ? textureUrl(player.cape_hash) : null;

  return (
    <Dialog
      open={open}
      title={`Upload Cape - ${player.name}`}
      onClose={onClose}
      footer={
        <>
          <Button variant="secondary" size="sm" onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <Button size="sm" onClick={handleConfirm} disabled={busy || !file}>
            {busy ? "..." : "Confirm Upload"}
          </Button>
        </>
      }
    >
      <div className="mb-4 flex gap-4">
        <div className="flex-1">
          <span className="mb-1.5 block text-xs text-neutral-500">Current</span>
          <SkinPreview
            skinUrl={currentSkinUrl}
            capeUrl={currentCapeUrl}
            skinModel={player.skin_model as "classic" | "slim"}
          />
        </div>
        <div className="flex-1">
          <span className="mb-1.5 block text-xs text-neutral-500">New</span>
          <SkinPreview
            skinUrl={currentSkinUrl}
            capeUrl={previewUrl}
            skinModel={player.skin_model as "classic" | "slim"}
          />
        </div>
      </div>

      <Input label="Cape File" type="file" accept="image/png" onChange={handleFileSelect} />

      {error && <p className="text-sm text-red-400">{error}</p>}
    </Dialog>
  );
}
