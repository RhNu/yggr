import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";

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
      setAuth: (token, clientToken) => set({ token, clientToken, authed: true }),
      logout: () => set({ token: null, clientToken: null, authed: false }),
    }),
    {
      name: "yggr-auth",
      storage: createJSONStorage(() => localStorage),
    },
  ),
);

export function getToken(): string | null {
  return useAuthStore.getState().token;
}

export function getClientToken(): string | null {
  return useAuthStore.getState().clientToken;
}
