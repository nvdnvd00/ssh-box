import { useCallback, useEffect, useRef, useState } from "react";
import { AppI18nStrings } from "../../../i18n";
import { HostEntry, VpsProbe } from "../../../types";
import { parseVpsError, vpsErrorText } from "./errors";

export interface VpsCtx {
  hostId: string;
  sudoPassword?: string;
  requestSudo: () => void;
  t: AppI18nStrings;
}

// Loads one studio's data; SUDO_REQUIRED opens the sudo prompt instead of showing an error.
export function useVpsResource<T>(vps: VpsCtx, load: (sudoPassword?: string) => Promise<T>) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const vpsRef = useRef(vps);
  vpsRef.current = vps;
  const loadRef = useRef(load);
  loadRef.current = load;

  const fail = useCallback((e: unknown) => {
    if (parseVpsError(e).code === "SUDO_REQUIRED") vpsRef.current.requestSudo();
    else setError(vpsErrorText(vpsRef.current.t, e));
  }, []);

  // A write can fail half-way (Windows enable: rule added, profile not switched): keep its error and show the host as it
  // is now. A lockout refusal or a missing sudo password means nothing ran.
  const failWrite = useCallback(
    (e: unknown) => {
      fail(e);
      const code = parseVpsError(e).code;
      if (code !== "LOCKOUT_BLOCKED" && code !== "SUDO_REQUIRED") {
        loadRef.current(vpsRef.current.sudoPassword).then(setData, () => {});
      }
    },
    [fail],
  );

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setData(await loadRef.current(vps.sudoPassword));
    } catch (e) {
      fail(e);
    } finally {
      setLoading(false);
    }
  }, [vps.hostId, vps.sudoPassword, fail]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const mutate = useCallback(
    async (run: (sudoPassword?: string) => Promise<T>) => {
      setError(null);
      try {
        setData(await run(vps.sudoPassword));
        return true;
      } catch (e) {
        failWrite(e);
        return false;
      }
    },
    [vps.sudoPassword, failWrite],
  );

  return { data, loading, error, setError, refresh, mutate, failWrite };
}

// Props every studio receives from VpsConfigWorkspace.
export interface VpsStudioProps {
  vps: VpsCtx;
  probe: VpsProbe;
  host: HostEntry;
  onSaveHost: (host: HostEntry, secret?: string) => Promise<void>;
}
