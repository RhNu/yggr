import { useEffect, useState } from "react";

import { textureUrl, type Player } from "@/api";
import SkinPreview from "@/components/SkinPreview";
import Button from "@/components/ui/Button";
import Dialog from "@/components/ui/Dialog";
import Input from "@/components/ui/Input";
import Select from "@/components/ui/Select";
import { useUploadTexture } from "@/queries";

interface Props {
  open: boolean;
  player: Player;
  onClose: () => void;
}

export default function UploadSkinDialog({ open, player, onClose }: Props) {
  const [file, setFile] = useState<File | null>(null);
  const [model, setModel] = useState(player.skin_model);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const uploadTexture = useUploadTexture();

  useEffect(() => {
    if (open) {
      setFile(null);
      setModel(player.skin_model);
      setPreviewUrl(null);
    }
  }, [open, player.skin_model]);

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

  const handleConfirm = () => {
    if (!file) return;
    uploadTexture.mutate(
      { playerId: player.id, type: "skin", file, model },
      { onSuccess: () => onClose() },
    );
  };

  const currentSkinUrl = player.skin_hash
    ? textureUrl(player.skin_hash)
    : `/service/textures/default/${player.skin_model}`;

  return (
    <Dialog
      open={open}
      title={`Upload Skin - ${player.name}`}
      onClose={onClose}
      footer={
        <>
          <Button
            variant="secondary"
            size="sm"
            onClick={onClose}
            disabled={uploadTexture.isPending}
          >
            Cancel
          </Button>
          <Button size="sm" onClick={handleConfirm} disabled={uploadTexture.isPending || !file}>
            {uploadTexture.isPending ? "..." : "Confirm Upload"}
          </Button>
        </>
      }
    >
      <div className="mb-4 flex gap-4">
        <div className="flex-1">
          <span className="mb-1.5 block text-xs text-neutral-500">Current</span>
          <SkinPreview
            skinUrl={currentSkinUrl}
            skinModel={player.skin_model as "classic" | "slim"}
          />
        </div>
        <div className="flex-1">
          <span className="mb-1.5 block text-xs text-neutral-500">New</span>
          <SkinPreview skinUrl={previewUrl} skinModel={model as "classic" | "slim"} />
        </div>
      </div>

      <Input label="Skin File" type="file" accept="image/png" onChange={handleFileSelect} />

      <Select label="Model" value={model} onChange={(e) => setModel(e.target.value)}>
        <option value="classic">Classic</option>
        <option value="slim">Slim</option>
      </Select>

      {uploadTexture.error && <p className="text-sm text-red-400">{uploadTexture.error.message}</p>}
    </Dialog>
  );
}
