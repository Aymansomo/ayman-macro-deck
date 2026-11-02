import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center h-full bg-[var(--color-bg-primary)] p-8 text-center">
          <span className="text-4xl mb-4">💥</span>
          <h1 className="text-lg font-semibold text-[var(--color-text-primary)] mb-2">
            Something went wrong
          </h1>
          <p className="text-sm text-[var(--color-text-muted)] mb-4 max-w-md">
            {this.state.error?.message ?? "An unexpected error occurred."}
          </p>
          <button
            onClick={() => window.location.reload()}
            className="px-4 py-2 rounded-lg text-sm font-medium bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white transition-colors cursor-pointer"
          >
            Reload App
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
