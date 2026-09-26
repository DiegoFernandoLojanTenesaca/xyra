/** Google's QR scanner, reached through the Android activity; see MainActivity.kt. */
interface QrBridge {
  scan(): void;
}

declare global {
  interface Window {
    XyraQr?: QrBridge;
    __xyraQrResult?: (raw: string | null) => void;
  }
}

/** The scanned text, or null when the player cancelled; outside Android it asks for the code as text, for testing. */
export function scanQr(fallbackPrompt: string): Promise<string | null> {
  const bridge = window.XyraQr;
  if (!bridge) return Promise.resolve(window.prompt(fallbackPrompt));
  return new Promise((resolve) => {
    window.__xyraQrResult = (raw) => {
      window.__xyraQrResult = undefined;
      resolve(raw);
    };
    bridge.scan();
  });
}
