import {
  createContext,
  useContext,
  useReducer,
  useEffect,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";

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

const BASE_URL_DEFAULT = "https://napi.mikawi.org";

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
    baseUrl: BASE_URL_DEFAULT,
    loading: true,
    error: null,
    flowToken: null,
  });

  useEffect(() => {
    invoke<boolean>("load_credential")
      .then((authed) => dispatch({ type: "RESTORED", authed }))
      .catch(() => dispatch({ type: "RESTORED", authed: false }));
  }, []);

  const login = async (username: string, password: string) => {
    dispatch({ type: "LOGIN_START" });
    try {
      // ponytail: login returns "" on success or "2FA_REQUIRED:<flow>" — never the token.
      const result = await invoke<string>("login", {
        username,
        password,
        baseUrl: state.baseUrl,
      });
      if (result.startsWith("2FA_REQUIRED:")) {
        dispatch({
          type: "LOGIN_2FA_REQUIRED",
          flowToken: result.slice("2FA_REQUIRED:".length),
        });
      } else {
        dispatch({ type: "LOGIN_SUCCESS" });
      }
    } catch (e) {
      dispatch({ type: "LOGIN_FAILURE", error: String(e) });
    }
  };

  const verify2fa = async (code: string) => {
    if (!state.flowToken) return;
    dispatch({ type: "LOGIN_START" });
    try {
      await invoke("verify_2fa", {
        flowToken: state.flowToken,
        code,
        baseUrl: state.baseUrl,
      });
      dispatch({ type: "LOGIN_SUCCESS" });
    } catch (e) {
      dispatch({ type: "LOGIN_FAILURE", error: String(e) });
    }
  };

  // ponytail: github_oauth blocks until the local callback server receives the token
  // (stored in the keyring server-side) or times out.
  const githubOAuth = async () => {
    dispatch({ type: "LOGIN_START" });
    try {
      await invoke("github_oauth", { baseUrl: state.baseUrl });
      dispatch({ type: "LOGIN_SUCCESS" });
    } catch (e) {
      dispatch({ type: "LOGIN_FAILURE", error: String(e) });
    }
  };

  const logout = () => {
    invoke("clear_credential").catch(() => {});
    dispatch({ type: "LOGOUT" });
  };

  const setBaseUrl = (url: string) => dispatch({ type: "SET_BASE_URL", url });

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
