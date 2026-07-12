import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { StatusEvent } from "./types";

export default function Capsule() {
  const [status, setStatus] = useState<StatusEvent>({
    state: "recording",
    message: "正在聆听",
  });

  useEffect(() => {
    const subscription = listen<StatusEvent>("dictation://state", ({ payload }) => {
      setStatus(payload);
    });
    return () => {
      void subscription.then((unlisten) => unlisten());
    };
  }, []);

  return (
    <main className={`capsule capsule--${status.state}`} data-tauri-drag-region>
      <span className="capsule__pulse" />
      <span>{status.message}</span>
    </main>
  );
}
