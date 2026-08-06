import { Route, Switch, useLocation } from "wouter";
import Login from "./pages/Login";
import Dashboard from "./pages/Dashboard";
import { useAuthStore } from "./store/authStore";

export default function App() {
  const authed = useAuthStore((s) => s.authed);
  const [, navigate] = useLocation();

  if (!authed) {
    return <Login />;
  }

  return (
    <Switch>
      <Route path="/login" component={() => <Login />} />
      <Route path="/" component={() => <Dashboard />} />
      <Route>
        <Dashboard />
      </Route>
    </Switch>
  );
}
