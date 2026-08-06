import { Component, type ReactNode } from "react";

import { createLogger } from "../../logger";

const log = createLogger("ErrorBoundary");

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: { componentStack: string }) {
    log.error("render error", { error: error.message, componentStack: info.componentStack });
  }

  handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="mx-auto mt-32 max-w-md px-6 text-center">
          <h2 className="mb-3 text-lg font-semibold text-neutral-100">Something went wrong</h2>
          <p className="mb-6 text-sm break-words text-neutral-500">
            {this.state.error?.message ?? "An unexpected error occurred."}
          </p>
          <button
            onClick={this.handleReload}
            className="cursor-pointer rounded-md bg-white px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-neutral-200"
          >
            Reload
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
