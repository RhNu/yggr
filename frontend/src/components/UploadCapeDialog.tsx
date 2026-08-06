import { useEffect, useState } from "react";
import Dialog from "./ui/Dialog";
import SkinPreview from "./SkinPreview";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { uploadTexture, textureUrl, type Player } from "../api";

interface Props {
  open: boolean;
  player: Player;
  onClose: () => void;
  onChanged: () => void;
}

export default function UploadCapeDialog({
  open,
  player,
  onClose,
  onChanged,
}: Props) {
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
  const currentCapeUrl = player.cape_hash
    ? textureUrl(player.cape_hash)
    : null;

  return (
    <Dialog
      open={open}
      title={`Upload Cape - ${player.name}`}
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
            onClick={handleConfirm}
            disabled={busy || !file}
          >
            {busy ? "..." : "Confirm Upload"}
          </button>
        </>
      }
    >
      <div className="skin-comparison">
        <div className="skin-comparison-item">
          <label>Current</label>
          <SkinPreview
            skinUrl={currentSkinUrl}
            capeUrl={currentCapeUrl}
            skinModel={player.skin_model as "classic" | "slim"}
          />
        </div>
        <div className="skin-comparison-item">
          <label>New</label>
          <SkinPreview
            skinUrl={currentSkinUrl}
            capeUrl={previewUrl}
            skinModel={player.skin_model as "classic" | "slim"}
          />
        </div>
      </div>

      <div className="form-group">
        <label>Cape File</label>
        <input type="file" accept="image/png" onChange={handleFileSelect} />
      </div>

      {error && <div className="error-msg">{error}</div>}
    </Dialog>
  );
}
