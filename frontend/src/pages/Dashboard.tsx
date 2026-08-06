import { useEffect, useState } from "react";
import { usePlayerStore } from "../store/playerStore";
import { useAuthStore } from "../store/authStore";
import PlayerCard from "../components/PlayerCard";
import CreatePlayerDialog from "../components/CreatePlayerDialog";
import Button from "../components/ui/Button";

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
      <div className="max-w-4xl mx-auto px-4 py-8">
        <p className="text-center text-neutral-500">Loading...</p>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      <header className="flex items-center justify-between mb-6 pb-4 border-b border-white/10">
        <h1 className="text-xl font-semibold text-neutral-100">YggR</h1>
        <div className="flex items-center gap-3">
          {me && (
            <span className="text-sm text-neutral-500">{me.username}</span>
          )}
          <Button variant="secondary" size="sm" onClick={handleLogout}>
            Logout
          </Button>
        </div>
      </header>

      {error && (
        <p className="text-sm text-red-400 mb-4">{error}</p>
      )}

      <div className="mb-6">
        <Button size="sm" onClick={() => setShowCreate(true)}>
          Add Character
        </Button>
      </div>

      {me && me.players.length > 0 ? (
        <div className="grid gap-4 [grid-template-columns:repeat(auto-fill,minmax(360px,1fr))]">
          {me.players.map((p) => (
            <PlayerCard key={p.id} player={p} onChanged={refresh} />
          ))}
        </div>
      ) : (
        !error && (
          <p className="text-center text-sm text-neutral-500 py-10">
            No players yet. Click "Add Character" to create one.
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
