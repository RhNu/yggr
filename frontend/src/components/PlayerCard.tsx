import { useState } from "react";

import type { Player } from "@/api";
import { deletePlayer, deleteTexture, textureUrl, updateSkinModel } from "@/api";
import SkinPreview from "@/components/SkinPreview";
import Button from "@/components/ui/Button";
import ConfirmDialog from "@/components/ui/ConfirmDialog";
import Select from "@/components/ui/Select";
import UploadCapeDialog from "@/components/UploadCapeDialog";
import UploadSkinDialog from "@/components/UploadSkinDialog";
import { useAsyncAction } from "@/hooks/useAsyncAction";

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
    <div className="rounded-xl border border-white/10 bg-white/[0.03] p-4 backdrop-blur-md">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-base font-medium text-neutral-100">{player.name}</h3>
        <Button variant="danger" size="sm" onClick={() => setShowDelete(true)} disabled={busy}>
          Delete
        </Button>
      </div>

      <SkinPreview
        skinUrl={skinUrl}
        capeUrl={capeUrl}
        skinModel={player.skin_model as "classic" | "slim"}
      />

      {error && <p className="mb-3 text-sm text-red-400">{error}</p>}

      <div className="mb-3 flex items-center gap-2">
        <span className="text-xs text-neutral-500">Model:</span>
        <Select
          value={player.skin_model}
          onChange={(e) => handleModelChange(e.target.value)}
          disabled={busy}
          className="w-auto px-2 py-1 text-xs"
        >
          <option value="classic">Classic</option>
          <option value="slim">Slim</option>
        </Select>
      </div>

      <div className="mb-2 flex flex-wrap gap-2">
        <Button
          variant="secondary"
          size="sm"
          onClick={() => setShowUploadSkin(true)}
          disabled={busy}
        >
          Upload Skin
        </Button>
        {player.skin_hash && (
          <Button
            variant="secondary"
            size="sm"
            onClick={() => handleDeleteTexture("skin")}
            disabled={busy}
          >
            Remove Skin
          </Button>
        )}
      </div>

      <div className="flex flex-wrap gap-2">
        <Button
          variant="secondary"
          size="sm"
          onClick={() => setShowUploadCape(true)}
          disabled={busy}
        >
          Upload Cape
        </Button>
        {player.cape_hash && (
          <Button
            variant="secondary"
            size="sm"
            onClick={() => handleDeleteTexture("cape")}
            disabled={busy}
          >
            Remove Cape
          </Button>
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
