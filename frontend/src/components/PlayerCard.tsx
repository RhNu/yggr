import { useState } from "react";
import type { Player } from "../api";
import {
  deletePlayer,
  deleteTexture,
  textureUrl,
  updateSkinModel,
} from "../api";
import SkinPreview from "./SkinPreview";
import UploadSkinDialog from "./UploadSkinDialog";
import UploadCapeDialog from "./UploadCapeDialog";
import ConfirmDialog from "./ui/ConfirmDialog";
import { useAsyncAction } from "../hooks/useAsyncAction";

interface Props {
  player: Player;
  onChanged: () => void;
}

export default function PlayerCard({ player, onChanged }: Props) {
  const [showUploadSkin, setShowUploadSkin] = useState(false);
  const [showUploadCape, setShowUploadCape] = useState(false);
  const [showDelete, setShowDelete] = useState(false);
  const { busy, error, run } = useAsyncAction();

  const handleModelChange = async (model: string) => {
    const ok = await run(() => updateSkinModel(player.id, model));
    if (ok !== null) onChanged();
  };

  const handleDeleteTexture = async (type: "skin" | "cape") => {
    const ok = await run(() => deleteTexture(player.id, type));
    if (ok !== null) onChanged();
  };

  const handleDeletePlayer = async () => {
    const ok = await run(() => deletePlayer(player.id));
    if (ok !== null) {
      setShowDelete(false);
      onChanged();
    }
  };

  const skinUrl = player.skin_hash
    ? textureUrl(player.skin_hash)
    : `/service/textures/default/${player.skin_model}`;
  const capeUrl = player.cape_hash ? textureUrl(player.cape_hash) : null;

  return (
    <div className="player-card">
      <div className="player-card-header">
        <h3>{player.name}</h3>
        <button
          className="danger btn-sm"
          onClick={() => setShowDelete(true)}
          disabled={busy}
        >
          Delete
        </button>
      </div>

      <SkinPreview
        skinUrl={skinUrl}
        capeUrl={capeUrl}
        skinModel={player.skin_model as "classic" | "slim"}
      />

      {error && <div className="error-msg">{error}</div>}

      <div className="info-row">
        <label>Model:</label>
        <select
          className="model-select"
          value={player.skin_model}
          onChange={(e) => handleModelChange(e.target.value)}
          disabled={busy}
        >
          <option value="classic">Classic</option>
          <option value="slim">Slim</option>
        </select>
      </div>

      <div className="player-card-actions">
        <button
          className="secondary btn-sm"
          onClick={() => setShowUploadSkin(true)}
          disabled={busy}
        >
          Upload Skin
        </button>
        {player.skin_hash && (
          <button
            className="secondary btn-sm"
            onClick={() => handleDeleteTexture("skin")}
            disabled={busy}
          >
            Remove Skin
          </button>
        )}
      </div>

      <div className="player-card-actions" style={{ marginTop: 8 }}>
        <button
          className="secondary btn-sm"
          onClick={() => setShowUploadCape(true)}
          disabled={busy}
        >
          Upload Cape
        </button>
        {player.cape_hash && (
          <button
            className="secondary btn-sm"
            onClick={() => handleDeleteTexture("cape")}
            disabled={busy}
          >
            Remove Cape
          </button>
        )}
      </div>

      <UploadSkinDialog
        open={showUploadSkin}
        player={player}
        onClose={() => setShowUploadSkin(false)}
        onChanged={onChanged}
      />

      <UploadCapeDialog
        open={showUploadCape}
        player={player}
        onClose={() => setShowUploadCape(false)}
        onChanged={onChanged}
      />

      <ConfirmDialog
        open={showDelete}
        title="Delete Player"
        message={`Delete player "${player.name}"?`}
        confirmLabel="Delete"
        busy={busy}
        onConfirm={handleDeletePlayer}
        onCancel={() => setShowDelete(false)}
      />
    </div>
  );
}
