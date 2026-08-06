import { useState } from "react";
import { login } from "../api";
import { useAuthStore } from "../store/authStore";
import Input from "../components/ui/Input";
import Button from "../components/ui/Button";

export default function Login() {
  const setAuth = useAuthStore((s) => s.setAuth);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const { access_token, client_token } = await login(username, password);
      setAuth(access_token, client_token);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center px-4">
      <div className="w-full max-w-sm rounded-xl border border-white/10 bg-neutral-900/60 backdrop-blur-md p-8 shadow-2xl">
        <h1 className="text-2xl font-bold text-center text-neutral-100 mb-8">
          YggR
        </h1>
        <form onSubmit={handleSubmit}>
          <Input
            label="Username"
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoFocus
            required
          />
          <Input
            label="Password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
          {error && (
            <p className="text-sm text-red-400 mb-4">{error}</p>
          )}
          <Button
            type="submit"
            disabled={loading}
            className="w-full"
          >
            {loading ? "..." : "Login"}
          </Button>
        </form>
      </div>
    </div>
  );
}
