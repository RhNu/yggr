import { Component, type ReactNode } from "react";

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

  handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="max-w-md mx-auto mt-32 text-center px-6">
          <h2 className="text-lg font-semibold text-neutral-100 mb-3">
            Something went wrong
          </h2>
          <p className="text-sm text-neutral-500 mb-6 break-words">
            {this.state.error?.message ?? "An unexpected error occurred."}
          </p>
          <button
            onClick={this.handleReload}
            className="rounded-md bg-white text-black px-4 py-2 text-sm font-medium hover:bg-neutral-200 transition-colors cursor-pointer"
          >
            Reload
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
