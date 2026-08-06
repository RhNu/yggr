import { useState } from "react";
import { Route, Switch, useLocation } from "wouter";
import Login from "./pages/Login";
import Dashboard from "./pages/Dashboard";
import { getToken } from "./store";

export default function App() {
  const [authed, setAuthed] = useState(!!getToken());
  const [, navigate] = useLocation();

  const handleLogin = () => {
    setAuthed(true);
    navigate("/");
  };

  const handleLogout = () => {
    setAuthed(false);
    navigate("/login");
  };

  if (!authed) {
    return <Login onLogin={handleLogin} />;
  }

  return (
    <Switch>
      <Route path="/login" component={() => <Login onLogin={handleLogin} />} />
      <Route path="/" component={() => <Dashboard onLogout={handleLogout} />} />
      <Route>
        <Dashboard onLogout={handleLogout} />
      </Route>
    </Switch>
  );
}
