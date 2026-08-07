import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  createPlayer as apiCreatePlayer,
  deletePlayer as apiDeletePlayer,
  deleteTexture as apiDeleteTexture,
  fetchMe,
  login as apiLogin,
  updateSkinModel as apiUpdateSkinModel,
  uploadTexture as apiUploadTexture,
} from "@/api";

export const meKey = ["me"] as const;

export function useMe() {
  return useQuery({
    queryKey: meKey,
    queryFn: fetchMe,
  });
}

export function useLogin() {
  return useMutation({
    mutationFn: ({ username, password }: { username: string; password: string }) =>
      apiLogin(username, password),
  });
}

export function useCreatePlayer() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ name, skinModel, uuid }: { name: string; skinModel: string; uuid?: string }) =>
      apiCreatePlayer(name, skinModel, uuid),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: meKey }),
  });
}

export function useDeletePlayer() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiDeletePlayer(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: meKey }),
  });
}

export function useUpdateSkinModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, model }: { id: string; model: string }) => apiUpdateSkinModel(id, model),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: meKey }),
  });
}

export function useUploadTexture() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      playerId,
      type,
      file,
      model,
    }: {
      playerId: string;
      type: "skin" | "cape";
      file: File;
      model?: string;
    }) => apiUploadTexture(playerId, type, file, model),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: meKey }),
  });
}

export function useDeleteTexture() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ playerId, type }: { playerId: string; type: "skin" | "cape" }) =>
      apiDeleteTexture(playerId, type),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: meKey }),
  });
}
