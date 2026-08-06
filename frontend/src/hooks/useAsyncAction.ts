import { useCallback, useState } from "react";

import { createLogger } from "@/logger";

const log = createLogger("useAsyncAction");

export function useAsyncAction() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const run = useCallback(async <T>(fn: () => Promise<T>): Promise<T | null> => {
    setError("");
    setBusy(true);
    try {
      return await fn();
    } catch (err) {
      log.warn("action failed", { error: err });
      setError(err instanceof Error ? err.message : "Operation failed");
      return null;
    } finally {
      setBusy(false);
    }
  }, []);

  const reset = useCallback(() => {
    setError("");
    setBusy(false);
  }, []);

  return { busy, error, run, reset };
}
