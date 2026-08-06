import { useState } from "react";
import { usePlayers } from "../hooks/usePlayers";
import PlayerCard from "../components/PlayerCard";
import CreatePlayerDialog from "../components/CreatePlayerDialog";
import { clearToken } from "../store";

interface Props {
  onLogout: () => void;
}

export default function Dashboard({ onLogout }: Props) {
  const { me, error, loading, refresh } = usePlayers(onLogout);
  const [showCreate, setShowCreate] = useState(false);

  const handleLogout = () => {
    clearToken();
    onLogout();
  };

  if (loading) {
    return (
      <div className="app">
        <p style={{ textAlign: "center", color: "var(--text-dim)" }}>
          Loading...
        </p>
      </div>
    );
  }

  return (
    <div className="app">
      <div className="header">
        <h1>Yggr</h1>
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <span style={{ fontSize: 13, color: "var(--text-dim)" }}>
            {me?.username}
          </span>
          <button className="secondary btn-sm" onClick={handleLogout}>
            Logout
          </button>
        </div>
      </div>

      {error && <div className="error-msg">{error}</div>}

      <div style={{ marginBottom: 16 }}>
        <button className="btn-sm" onClick={() => setShowCreate(true)}>
          Add Character
        </button>
      </div>

      {me && me.players.length > 0 ? (
        <div className="player-grid">
          {me.players.map((p) => (
            <PlayerCard key={p.id} player={p} onChanged={refresh} />
          ))}
        </div>
      ) : (
        !error && (
          <div className="no-skin">
            No players yet. Click "Add Character" to create one.
          </div>
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
