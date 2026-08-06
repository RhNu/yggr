import { getToken } from "./store";

const API_BASE = "";

async function request(path: string, options: RequestInit = {}) {
  const token = getToken();
  const headers: Record<string, string> = {
    ...(options.headers as Record<string, string>),
  };
  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }
  if (options.body && !(options.body instanceof FormData)) {
    headers["Content-Type"] = "application/json";
  }
  const resp = await fetch(`${API_BASE}${path}`, { ...options, headers });
  if (resp.status === 204) return null;
  const text = await resp.text();
  if (!resp.ok) {
    let msg = `HTTP ${resp.status}`;
    try {
      const j = JSON.parse(text);
      msg = j.errorMessage || j.error || msg;
    } catch {
      if (text) msg = text;
    }
    throw new Error(msg);
  }
  return text ? JSON.parse(text) : null;
}

export interface Player {
  id: string;
  name: string;
  skin_hash: string | null;
  cape_hash: string | null;
  skin_model: string;
}

export interface MeResponse {
  username: string;
  preferred_language: string;
  players: Player[];
}

export async function login(
  username: string,
  password: string
): Promise<{ access_token: string; client_token: string }> {
  const data = await request("/service/authserver/authenticate", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  });
  return {
    access_token: data.accessToken,
    client_token: data.clientToken,
  };
}

export async function fetchMe(): Promise<MeResponse> {
  return request("/api/me");
}

export async function createPlayer(
  name: string,
  skinModel: string
): Promise<Player> {
  return request("/api/players", {
    method: "POST",
    body: JSON.stringify({ name, skin_model: skinModel }),
  });
}

export async function deletePlayer(id: string): Promise<void> {
  await request(`/api/players/${id}`, { method: "DELETE" });
}

export async function updateSkinModel(
  id: string,
  model: string
): Promise<void> {
  await request(`/api/players/${id}/skin-model`, {
    method: "PUT",
    body: JSON.stringify({ model }),
  });
}

export async function uploadTexture(
  playerId: string,
  type: "skin" | "cape",
  file: File,
  model?: string
): Promise<void> {
  const form = new FormData();
  form.append("file", file);
  if (type === "skin" && model) {
    form.append("model", model);
  }
  await request(`/service/api/user/profile/${playerId}/${type}`, {
    method: "PUT",
    body: form,
  });
}

export async function deleteTexture(
  playerId: string,
  type: "skin" | "cape"
): Promise<void> {
  await request(`/service/api/user/profile/${playerId}/${type}`, {
    method: "DELETE",
  });
}

export function textureUrl(hash: string): string {
  return `/service/textures/${hash}`;
}
