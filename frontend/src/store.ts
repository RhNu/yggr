const TOKEN_KEY = "yggr_token";
const CLIENT_TOKEN_KEY = "yggr_client_token";

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string, clientToken: string) {
  localStorage.setItem(TOKEN_KEY, token);
  localStorage.setItem(CLIENT_TOKEN_KEY, clientToken);
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(CLIENT_TOKEN_KEY);
}

export function getClientToken(): string | null {
  return localStorage.getItem(CLIENT_TOKEN_KEY);
}
