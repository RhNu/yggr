const LEVELS = { debug: 10, info: 20, warn: 30, error: 40 } as const;

type Level = keyof typeof LEVELS;

const minLevel: Level = import.meta.env.DEV ? "debug" : "info";

function emit(level: Level, scope: string, msg: string, ctx?: Record<string, unknown>) {
  if (LEVELS[level] < LEVELS[minLevel]) return;
  const tag = `[${scope}] ${msg}`;
  if (ctx) {
    console[level](tag, ctx);
  } else {
    console[level](tag);
  }
}

export function createLogger(scope: string) {
  return {
    debug: (msg: string, ctx?: Record<string, unknown>) => emit("debug", scope, msg, ctx),
    info: (msg: string, ctx?: Record<string, unknown>) => emit("info", scope, msg, ctx),
    warn: (msg: string, ctx?: Record<string, unknown>) => emit("warn", scope, msg, ctx),
    error: (msg: string, ctx?: Record<string, unknown>) => emit("error", scope, msg, ctx),
  };
}

export type Logger = ReturnType<typeof createLogger>;
