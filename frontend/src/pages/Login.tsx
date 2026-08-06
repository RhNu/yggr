import { useState } from "react";

import Button from "@/components/ui/Button";
import Input from "@/components/ui/Input";
import { useLogin } from "@/queries";
import { useAuthStore } from "@/store/authStore";

export default function Login() {
  const setAuth = useAuthStore((s) => s.setAuth);
  const loginMutation = useLogin();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    loginMutation.mutate(
      { username, password },
      {
        onSuccess: (data) => {
          setAuth(data.access_token, data.client_token);
        },
      },
    );
  };

  return (
    <div className="flex min-h-screen items-center justify-center px-4">
      <div className="w-full max-w-sm rounded-xl border border-white/10 bg-neutral-900/60 p-8 shadow-2xl backdrop-blur-md">
        <h1 className="mb-8 text-center text-2xl font-bold text-neutral-100">YggR</h1>
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
          {loginMutation.error && (
            <p className="mb-4 text-sm text-red-400">{loginMutation.error.message}</p>
          )}
          <Button type="submit" disabled={loginMutation.isPending} className="w-full">
            {loginMutation.isPending ? "..." : "Login"}
          </Button>
        </form>
      </div>
    </div>
  );
}
