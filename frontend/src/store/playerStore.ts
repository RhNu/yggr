import { create } from "zustand";
import {
  fetchMe,
  createPlayer as apiCreatePlayer,
  deletePlayer as apiDeletePlayer,
  updateSkinModel as apiUpdateSkinModel,
  uploadTexture as apiUploadTexture,
  deleteTexture as apiDeleteTexture,
  type MeResponse,
  type Player,
} from "../api";
import { useAuthStore } from "./authStore";

interface PlayerState {
  me: MeResponse | null;
  loading: boolean;
  error: string;
  refresh: () => Promise<void>;
  addPlayer: (name: string, model: string) => Promise<boolean>;
  removePlayer: (id: string) => Promise<boolean>;
  setSkinModel: (id: string, model: string) => Promise<boolean>;
  uploadSkin: (
    playerId: string,
    file: File,
    model?: string
  ) => Promise<boolean>;
  uploadCape: (playerId: string, file: File) => Promise<boolean>;
  removeTexture: (
    playerId: string,
    type: "skin" | "cape"
  ) => Promise<boolean>;
  clear: () => void;
}

export const usePlayerStore = create<PlayerState>((set, get) => ({
  me: null,
  loading: false,
  error: "",

  refresh: async () => {
    set({ loading: true, error: "" });
    try {
      const data = await fetchMe();
      set({ me: data, loading: false });
    } catch (err) {
      if (
        err instanceof Error &&
        err.message.includes("Unauthorized")
      ) {
        useAuthStore.getState().logout();
        set({ me: null, loading: false });
        return;
      }
      set({
        error: err instanceof Error ? err.message : "Failed to load data",
        loading: false,
      });
    }
  },

  addPlayer: async (name, model) => {
    try {
      const player = await apiCreatePlayer(name, model);
      const me = get().me;
      if (me) {
        set({ me: { ...me, players: [...me.players, player] } });
      }
      return true;
    } catch {
      return false;
    }
  },

  removePlayer: async (id) => {
    try {
      await apiDeletePlayer(id);
      const me = get().me;
      if (me) {
        set({
          me: { ...me, players: me.players.filter((p) => p.id !== id) },
        });
      }
      return true;
    } catch {
      return false;
    }
  },

  setSkinModel: async (id, model) => {
    try {
      await apiUpdateSkinModel(id, model);
      const me = get().me;
      if (me) {
        set({
          me: {
            ...me,
            players: me.players.map((p) =>
              p.id === id ? { ...p, skin_model: model } : p
            ),
          },
        });
      }
      return true;
    } catch {
      return false;
    }
  },

  uploadSkin: async (playerId, file, model) => {
    try {
      await apiUploadTexture(playerId, "skin", file, model);
      await get().refresh();
      return true;
    } catch {
      return false;
    }
  },

  uploadCape: async (playerId, file) => {
    try {
      await apiUploadTexture(playerId, "cape", file);
      await get().refresh();
      return true;
    } catch {
      return false;
    }
  },

  removeTexture: async (playerId, type) => {
    try {
      await apiDeleteTexture(playerId, type);
      const me = get().me;
      if (me) {
        set({
          me: {
            ...me,
            players: me.players.map((p) =>
              p.id === playerId
                ? {
                    ...p,
                    [type === "skin" ? "skin_hash" : "cape_hash"]: null,
                  }
                : p
            ) as Player[],
          },
        });
      }
      return true;
    } catch {
      return false;
    }
  },

  clear: () => set({ me: null, loading: false, error: "" }),
}));
