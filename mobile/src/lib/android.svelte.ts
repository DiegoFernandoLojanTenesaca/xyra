import type { Pairing } from './pairing';

const REPOSITORY_API = 'https://api.github.com/repos/DiegoFernandoLojanTenesaca/xyra';
const APK_EXTENSION = '.apk';
const SHA256_PREFIX = 'sha256:';
const WATCH_KEY = 'xyra-watch';
const WATCH_OFF = 'off';

/** The Android side of the app; see MainActivity.kt and MatchWatch.kt. */
interface WatchBridge {
  start(settings: string): void;
  stop(): void;
  test(): void;
}

interface NetworkBridge {
  current(): string;
}

/** The phone's Wi-Fi address and whether it is on Wi-Fi. */
export interface PhoneNetwork {
  address: string;
  wifi: boolean;
}

interface UpdateBridge {
  version(): string;
  install(url: string, sha256: string): void;
}

type UpdateStatus = 'progress' | 'installing' | 'failed';

declare global {
  interface Window {
    XyraWatch?: WatchBridge;
    XyraUpdate?: UpdateBridge;
    XyraNetwork?: NetworkBridge;
    __xyraUpdate?: (status: UpdateStatus, percent: number) => void;
  }
}

/** The texts the waiting notification and the match notification show, in the PC's language. */
export type WatchTexts = Record<
  | 'watchingChannel'
  | 'matchChannel'
  | 'watching'
  | 'watchingText'
  | 'matchFound'
  | 'matchText'
  | 'accept'
  | 'accepted'
  | 'acceptFailed'
  | 'decline'
  | 'declined'
  | 'champSelect'
  | 'champSelectText'
  | 'testDone',
  string
>;

interface PhoneRelease {
  version: string;
  url: string;
  sha256: string;
}

interface ReleaseAnswer {
  tag_name: string;
  assets: { name: string; browser_download_url: string; digest: string | null }[];
}

const versionNumbers = (version: string) =>
  version
    .replace(/^v/, '')
    .split('.')
    .map((part) => Number.parseInt(part, 10) || 0);

/** Whether `candidate` is a later version than `current`, comparing each number in turn. */
export function isNewer(candidate: string, current: string) {
  const [next, now] = [versionNumbers(candidate), versionNumbers(current)];
  for (let i = 0; i < Math.max(next.length, now.length); i++) {
    if ((next[i] ?? 0) !== (now[i] ?? 0)) return (next[i] ?? 0) > (now[i] ?? 0);
  }
  return false;
}

/** What only the Android app can do: wait for matches with the app closed and update itself. */
class Android {
  readonly version = window.XyraUpdate?.version() ?? null;
  release = $state<PhoneRelease | null>(null);
  checking = $state(false);
  progress = $state<number | null>(null);
  failed = $state(false);
  watching = $state(localStorage.getItem(WATCH_KEY) !== WATCH_OFF);

  get available() {
    return this.version !== null;
  }

  /** Looks for a newer APK among the releases on GitHub. */
  check = async () => {
    if (!this.version) return;
    this.checking = true;
    try {
      const answer: ReleaseAnswer = await (await fetch(`${REPOSITORY_API}/releases/latest`)).json();
      const apk = answer.assets.find((asset) => asset.name.toLowerCase().endsWith(APK_EXTENSION));
      this.release =
        apk && isNewer(answer.tag_name, this.version)
          ? { version: answer.tag_name.replace(/^v/, ''), url: apk.browser_download_url, sha256: apk.digest?.replace(SHA256_PREFIX, '') ?? '' }
          : null;
    } catch {
      this.release = null;
    }
    this.checking = false;
  };

  install = () => {
    const release = this.release;
    if (!release || !window.XyraUpdate) return;
    this.failed = false;
    this.progress = 0;
    window.__xyraUpdate = (status, percent) => {
      this.progress = status === 'progress' ? percent : null;
      this.failed = status === 'failed';
    };
    window.XyraUpdate.install(release.url, release.sha256);
  };

  /** The phone's network; null outside Android. */
  network = (): PhoneNetwork | null => {
    const bridge = window.XyraNetwork;
    return bridge ? JSON.parse(bridge.current()) : null;
  };

  /** Rings the match notification once, as it will when a match is found. */
  testWatch = () => window.XyraWatch?.test();

  setWatching = (on: boolean) => {
    localStorage.setItem(WATCH_KEY, on ? '' : WATCH_OFF);
    this.watching = on;
  };

  /** Keeps the wait for matches in step with the pairing and the player's choice. */
  follow = (pairing: Pairing | null, texts: WatchTexts) => {
    const bridge = window.XyraWatch;
    if (!bridge) return;
    if (pairing && this.watching) bridge.start(JSON.stringify({ hosts: pairing.hosts, port: pairing.port, token: pairing.token, texts }));
    else bridge.stop();
  };
}

export const android = new Android();
