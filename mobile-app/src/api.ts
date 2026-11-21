export interface DeckButton {
  id: string;
  key: number;
  name: string;
  icon: string | null;
  description: string | null;
  enabled: boolean;
  actionCount: number;
}

export interface DeckProfile {
  id: string;
  name: string;
}

export interface DeckState {
  app: string;
  version: string;
  gamingMode: boolean;
  mode: string;
  profileId: string;
  profileName: string;
  profiles: DeckProfile[];
  buttons: DeckButton[];
  buttonCount: number;
}

export interface Connection {
  host: string;
  port: number;
  token: string;
}

const STORAGE_KEY = "ayman-deck.connection";

export function loadConnection(): Connection | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<Connection>;
    if (!parsed.host || !parsed.port || !parsed.token) return null;
    return { host: parsed.host, port: parsed.port, token: parsed.token };
  } catch {
    return null;
  }
}

export function saveConnection(connection: Connection) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(connection));
}

export function clearConnection() {
  localStorage.removeItem(STORAGE_KEY);
}

/** Accepts "192.168.1.10", "192.168.1.10:8787", "http://192.168.1.10:8787/" ... */
export function parseHostInput(input: string): { host: string; port?: number } {
  let value = input.trim().replace(/^https?:\/\//i, "").replace(/\/.*$/, "");
  let port: number | undefined;
  const match = value.match(/^([^:]+):(\d+)$/);
  if (match) {
    value = match[1];
    port = Number(match[2]);
  }
  return { host: value, port };
}

function baseUrl(connection: Pick<Connection, "host" | "port">): string {
  return `http://${connection.host}:${connection.port}`;
}

async function request<T>(
  connection: Pick<Connection, "host" | "port">,
  path: string,
  init: RequestInit = {},
  timeoutMs = 6000,
): Promise<T> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(`${baseUrl(connection)}${path}`, {
      ...init,
      signal: controller.signal,
    });
    const text = await response.text();
    const data = text ? JSON.parse(text) : {};
    if (!response.ok) {
      throw new Error(
        (data as { error?: string }).error ?? `PC returned ${response.status}`,
      );
    }
    return data as T;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      throw new Error("PC did not answer — check WiFi and that the app is open");
    }
    if (error instanceof TypeError) {
      throw new Error("Cannot reach the PC — check the address");
    }
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

export async function ping(host: string, port: number): Promise<void> {
  await request<{ ok: boolean }>({ host, port }, "/api/ping", {}, 4000);
}

export async function pair(
  host: string,
  port: number,
  code: string,
): Promise<string> {
  const result = await request<{ token: string }>(
    { host, port },
    "/api/pair",
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code }),
    },
    8000,
  );
  if (!result.token) throw new Error("PC did not return a token");
  return result.token;
}

function authHeaders(connection: Connection): HeadersInit {
  return {
    "Content-Type": "application/json",
    "X-Deck-Token": connection.token,
  };
}

export async function fetchState(connection: Connection): Promise<DeckState> {
  return request<DeckState>(connection, "/api/state", {
    headers: authHeaders(connection),
  });
}

export interface PressResult {
  ok: boolean;
  button: string;
  results: string[];
  state: DeckState | null;
}

export async function pressButton(
  connection: Connection,
  buttonId: string,
): Promise<PressResult> {
  return request<PressResult>(
    connection,
    "/api/press",
    {
      method: "POST",
      headers: authHeaders(connection),
      body: JSON.stringify({ buttonId }),
    },
    20000,
  );
}
