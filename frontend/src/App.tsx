import { Route, Switch } from "wouter";

import Dashboard from "@/pages/Dashboard";
import Login from "@/pages/Login";
import { useAuthStore } from "@/store/authStore";

export default function App() {
  const authed = useAuthStore((s) => s.authed);

  if (!authed) {
    return <Login />;
  }

  return (
    <Switch>
      <Route path="/login" component={Login} />
      <Route path="/" component={Dashboard} />
      <Route>
        <Dashboard />
      </Route>
    </Switch>
  );
}
