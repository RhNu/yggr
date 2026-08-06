import { useCallback, useEffect, useState } from "react";
import { fetchMe, type MeResponse } from "../api";
import { clearToken } from "../store";

export function usePlayers(onLogout: () => void) {
  const [me, setMe] = useState<MeResponse | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setError("");
    try {
      const data = await fetchMe();
      setMe(data);
    } catch (err) {
      if (err instanceof Error && err.message.includes("Unauthorized")) {
        clearToken();
        onLogout();
        return;
      }
      setError(err instanceof Error ? err.message : "Failed to load data");
    } finally {
      setLoading(false);
    }
  }, [onLogout]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { me, error, loading, refresh };
}
