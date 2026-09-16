import {
  createContext,
  useContext,
  useReducer,
  useEffect,
  type ReactNode,
} from "react";
import { invoke } from "./tauri.ts";

interface AuthState {
  authed: boolean;
  baseUrl: string;
  loading: boolean;
  error: string | null;
  flowToken: string | null;
}

type AuthAction =
  | { type: "LOGIN_START" }
  | { type: "LOGIN_SUCCESS" }
  | { type: "LOGIN_2FA_REQUIRED"; flowToken: string }
  | { type: "LOGIN_FAILURE"; error: string }
  | { type: "LOGOUT" }
  | { type: "RESTORED"; authed: boolean }
  | { type: "SET_BASE_URL"; url: string };

const BASE_URL_DEFAULT = "https://tal2a.app";

const BASE_URL_KEY = "napi:base-url";

function canonicalizeBaseUrl(url: string): string {
  const t = url.trim();
  if (!t || t.includes("napi.mikawi.org")) return BASE_URL_DEFAULT;
  return t;
}

function readBaseUrl(): string {
  try {
    const stored = localStorage.getItem(BASE_URL_KEY) || "";
    const next = canonicalizeBaseUrl(stored);
    if (stored && stored !== next) {
      localStorage.setItem(BASE_URL_KEY, next);
    }
    return next;
  } catch {
    return BASE_URL_DEFAULT;
  }
}

// Tauri IPC errors arrive as objects; String(e) gives "[object Object]".
function errorText(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

function authReducer(state: AuthState, action: AuthAction): AuthState {
  switch (action.type) {
    case "LOGIN_START":
      return { ...state, loading: true, error: null, flowToken: null };
    case "LOGIN_SUCCESS":
      return {
        ...state,
        authed: true,
        loading: false,
        error: null,
        flowToken: null,
      };
    case "LOGIN_2FA_REQUIRED":
      return {
        ...state,
        loading: false,
        error: null,
        flowToken: action.flowToken,
      };
    case "LOGIN_FAILURE":
      return { ...state, authed: false, loading: false, error: action.error };
    case "LOGOUT":
      return { ...state, authed: false, error: null, flowToken: null };
    case "RESTORED":
      return { ...state, authed: action.authed, loading: false };
    case "SET_BASE_URL":
      return { ...state, baseUrl: action.url };
    default:
      return state;
  }
}

interface AuthContextValue {
  state: AuthState;
  login: (username: string, password: string) => Promise<void>;
  verify2fa: (code: string) => Promise<void>;
  githubOAuth: () => Promise<void>;
  logout: () => void;
  setBaseUrl: (url: string) => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(authReducer, {
    authed: false,
    baseUrl: readBaseUrl(),
    loading: true,
    error: null,
    flowToken: null,
  });

  useEffect(() => {
    invoke<boolean>("load_credential")
      .then((authed) => dispatch({ type: "RESTORED", authed }))
      .catch(() => dispatch({ type: "RESTORED", authed: false }));
  }, []);

  // ponytail: password and GitHub sign-in share a response contract — "" on
  // success, "2FA_REQUIRED:<flow>" when the server wants a second factor. The
  // server applies the same login policy to both, so one handler covers both.
  const applyLoginResult = (result: string) => {
    if (result.startsWith("2FA_REQUIRED:")) {
      dispatch({
        type: "LOGIN_2FA_REQUIRED",
        flowToken: result.slice("2FA_REQUIRED:".length),
      });
    } else {
      dispatch({ type: "LOGIN_SUCCESS" });
    }
  };

  const login = async (username: string, password: string) => {
    dispatch({ type: "LOGIN_START" });
    try {
      applyLoginResult(
        await invoke<string>("login", {
          username,
          password,
          baseUrl: state.baseUrl,
        }),
      );
    } catch (e) {
      dispatch({ type: "LOGIN_FAILURE", error: errorText(e) });
    }
  };

  const verify2fa = async (code: string) => {
    if (!state.flowToken) return;
    dispatch({ type: "LOGIN_START" });
    try {
      applyLoginResult(
        await invoke<string>("verify_2fa", {
        flowToken: state.flowToken,
        code,
        baseUrl: state.baseUrl,
        }),
      );
    } catch (e) {
      dispatch({ type: "LOGIN_FAILURE", error: errorText(e) });
    }
  };
  const setBaseUrl = (url: string) => {
    const next = canonicalizeBaseUrl(url);
    try {
      localStorage.setItem(BASE_URL_KEY, next);
    } catch {
      // localStorage unavailable — session-only
    }
    invoke("save_base_url", { baseUrl: next }).catch(() => {});
    dispatch({ type: "SET_BASE_URL", url: next });
  };

  // ponytail: github_oauth blocks on a loopback callback, then returns the same
  // contract as login — the server runs the same login policy for both.
  const githubOAuth = async () => {
    dispatch({ type: "LOGIN_START" });
    try {
      applyLoginResult(
        await invoke<string>("github_oauth", { baseUrl: state.baseUrl }),
      );
    } catch (e) {
      dispatch({ type: "LOGIN_FAILURE", error: errorText(e) });
    }
  };

  const logout = () => {
    invoke("clear_credential").catch(() => {});
    dispatch({ type: "LOGOUT" });
  };

  return (
    <AuthContext.Provider
      value={{ state, login, verify2fa, githubOAuth, logout, setBaseUrl }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used inside AuthProvider");
  return ctx;
}
