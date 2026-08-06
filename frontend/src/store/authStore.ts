import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";

import { createLogger } from "@/logger";

const log = createLogger("authStore");

interface AuthState {
  token: string | null;
  clientToken: string | null;
  authed: boolean;
  setAuth: (token: string, clientToken: string) => void;
  logout: () => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      token: null,
      clientToken: null,
      authed: false,
      setAuth: (token, clientToken) => {
        log.info("authenticated");
        set({ token, clientToken, authed: true });
      },
      logout: () => {
        log.info("logged out");
        set({ token: null, clientToken: null, authed: false });
      },
    }),
    {
      name: "yggr-auth",
      storage: createJSONStorage(() => localStorage),
    },
  ),
);
