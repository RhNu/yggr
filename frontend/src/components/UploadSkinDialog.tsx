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

export default function UploadSkinDialog({
  open,
  player,
  onClose,
  onChanged,
}: Props) {
  const [file, setFile] = useState<File | null>(null);
  const [model, setModel] = useState(player.skin_model);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const { busy, error, run } = useAsyncAction();

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

  const handleConfirm = async () => {
    if (!file) return;
    const result = await run(() =>
      uploadTexture(player.id, "skin", file, model)
    );
    if (result !== null) {
      onClose();
      onChanged();
    }
  };

  const currentSkinUrl = player.skin_hash
    ? textureUrl(player.skin_hash)
    : null;

  return (
    <Dialog
      open={open}
      title={`Upload Skin - ${player.name}`}
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
            skinModel={player.skin_model as "classic" | "slim"}
          />
        </div>
        <div className="skin-comparison-item">
          <label>New</label>
          <SkinPreview
            skinUrl={previewUrl}
            skinModel={model as "classic" | "slim"}
          />
        </div>
      </div>

      <div className="form-group">
        <label>Skin File</label>
        <input type="file" accept="image/png" onChange={handleFileSelect} />
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
