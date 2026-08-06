import { useEffect, useState } from "react";

import CreatePlayerDialog from "@/components/CreatePlayerDialog";
import PlayerCard from "@/components/PlayerCard";
import Button from "@/components/ui/Button";
import { useAuthStore } from "@/store/authStore";
import { usePlayerStore } from "@/store/playerStore";

export default function Dashboard() {
  const { me, error, loading, refresh } = usePlayerStore();
  const logout = useAuthStore((s) => s.logout);
  const [showCreate, setShowCreate] = useState(false);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleLogout = () => {
    logout();
  };

  if (loading && !me) {
    return (
      <div className="mx-auto max-w-4xl px-4 py-8">
        <p className="text-center text-neutral-500">Loading...</p>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-4xl px-4 py-8">
      <header className="mb-6 flex items-center justify-between border-b border-white/10 pb-4">
        <h1 className="text-xl font-semibold text-neutral-100">YggR</h1>
        <div className="flex items-center gap-3">
          {me && <span className="text-sm text-neutral-500">{me.username}</span>}
          <Button variant="secondary" size="sm" onClick={handleLogout}>
            Logout
          </Button>
        </div>
      </header>

      {error && <p className="mb-4 text-sm text-red-400">{error}</p>}

      <div className="mb-6">
        <Button size="sm" onClick={() => setShowCreate(true)}>
          Add Character
        </Button>
      </div>

      {me && me.players.length > 0 ? (
        <div className="grid [grid-template-columns:repeat(auto-fill,minmax(360px,1fr))] gap-4">
          {me.players.map((p) => (
            <PlayerCard key={p.id} player={p} onChanged={refresh} />
          ))}
        </div>
      ) : (
        !error && (
          <p className="py-10 text-center text-sm text-neutral-500">
            No players yet. Click &quot;Add Character&quot; to create one.
          </p>
        )
      )}

      <CreatePlayerDialog
        open={showCreate}
        onClose={() => setShowCreate(false)}
        onCreated={refresh}
      />
    </div>
  );
}
