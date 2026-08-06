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
import { createLogger } from "../logger";
import { useAuthStore } from "./authStore";

const log = createLogger("playerStore");

interface PlayerState {
  me: MeResponse | null;
  loading: boolean;
  error: string;
  refresh: () => Promise<void>;
  addPlayer: (name: string, model: string) => Promise<boolean>;
  removePlayer: (id: string) => Promise<boolean>;
  setSkinModel: (id: string, model: string) => Promise<boolean>;
  uploadSkin: (playerId: string, file: File, model?: string) => Promise<boolean>;
  uploadCape: (playerId: string, file: File) => Promise<boolean>;
  removeTexture: (playerId: string, type: "skin" | "cape") => Promise<boolean>;
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
      if (err instanceof Error && err.message.includes("Unauthorized")) {
        log.warn("session expired");
        useAuthStore.getState().logout();
        set({ me: null, loading: false });
        return;
      }
      log.error("failed to load data", { error: err });
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
      log.debug("player created", { name });
      return true;
    } catch (err) {
      log.error("failed to create player", { name, error: err });
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
      log.debug("player deleted", { id });
      return true;
    } catch (err) {
      log.error("failed to delete player", { id, error: err });
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
            players: me.players.map((p) => (p.id === id ? { ...p, skin_model: model } : p)),
          },
        });
      }
      log.debug("skin model updated", { id, model });
      return true;
    } catch (err) {
      log.error("failed to update skin model", { id, error: err });
      return false;
    }
  },

  uploadSkin: async (playerId, file, model) => {
    try {
      await apiUploadTexture(playerId, "skin", file, model);
      await get().refresh();
      log.debug("skin uploaded", { playerId });
      return true;
    } catch (err) {
      log.error("failed to upload skin", { playerId, error: err });
      return false;
    }
  },

  uploadCape: async (playerId, file) => {
    try {
      await apiUploadTexture(playerId, "cape", file);
      await get().refresh();
      log.debug("cape uploaded", { playerId });
      return true;
    } catch (err) {
      log.error("failed to upload cape", { playerId, error: err });
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
                : p,
            ) as Player[],
          },
        });
      }
      log.debug("texture removed", { playerId, type });
      return true;
    } catch (err) {
      log.error("failed to remove texture", { playerId, type, error: err });
      return false;
    }
  },

  clear: () => set({ me: null, loading: false, error: "" }),
}));
