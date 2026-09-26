import type { AppError, EngineState, PcInfo } from '$shared/types';
import { address, type Pairing, type Params } from './pairing';

export { parsePairing, type Pairing } from './pairing';

const STORAGE_KEY = 'xyra-pairing';
const REACH_TIMEOUT_MS = 2500;
/** A state request waits up to 20 s for a change on the PC; this leaves room for a slow network. */
const STATE_TIMEOUT_MS = 30000;
const RETRY_MS = 3000;
const FORBIDDEN = 403;

export type LinkStatus = 'connecting' | 'online' | 'offline' | 'codeChanged';

type Answer<T> = { ok: true; value: T } | { ok: false; error: AppError };

/** A failure the PC reported, shown as `errors:<code>`. */
export class LinkError extends Error {
  constructor(readonly failure: AppError) {
    super(failure.code);
  }
}

/** The phone model Android reports, like "Infinix X6816C", so the PC can tell phones apart. */
function deviceName() {
  return /Android [^;]+; ([^;)]+?)(?: Build\/|\))/.exec(navigator.userAgent)?.[1] ?? 'Android';
}

function saved(): Pairing | null {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null');
  } catch {
    return null;
  }
}

/** The connection with Xyra on the PC: finds it on the home network or Tailscale and follows its state. */
class Link {
  pairing = $state<Pairing | null>(saved());
  status = $state<LinkStatus>('connecting');
  pc = $state<PcInfo | null>(null);
  state = $state<EngineState | null>(null);
  #base: string | null = null;
  #session = 0;
  #retry: ReturnType<typeof setTimeout> | undefined;

  /** Registers this phone with the PC that showed the code; the phone gets its own token, which the PC can revoke. */
  pair = async (code: Pairing) => {
    for (const host of code.hosts) {
      const base = `http://${host}:${code.port}`;
      let answer: Answer<{ token: string }>;
      try {
        const response = await fetch(address(base, '/api/pair', code.token, { name: deviceName() }), {
          method: 'POST',
          signal: AbortSignal.timeout(REACH_TIMEOUT_MS),
        });
        answer = await response.json();
      } catch {
        continue;
      }
      if (!answer.ok) throw new LinkError(answer.error);
      const pairing = { ...code, token: answer.value.token };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(pairing));
      this.pairing = pairing;
      this.connect();
      return;
    }
    throw new Error('unreachable');
  };

  forget = () => {
    localStorage.removeItem(STORAGE_KEY);
    this.#stop();
    this.pairing = null;
    this.pc = null;
    this.state = null;
  };

  /** Tries each address of the PC, the home network first, and follows the first one that answers. */
  connect = async () => {
    const pairing = this.pairing;
    if (!pairing) return;
    const session = this.#stop();
    this.status = 'connecting';
    for (const host of pairing.hosts) {
      const base = `http://${host}:${pairing.port}`;
      try {
        const response = await fetch(address(base, '/api/pc', pairing.token), { signal: AbortSignal.timeout(REACH_TIMEOUT_MS) });
        if (session !== this.#session) return;
        if (response.status === FORBIDDEN) {
          this.status = 'codeChanged';
          return;
        }
        if (!response.ok) continue;
        this.pc = await response.json();
        this.#base = base;
        this.status = 'online';
        this.#follow(session);
        return;
      } catch {
        continue;
      }
    }
    if (session === this.#session) this.#lost(session);
  };

  /** Reads data the PC answers as is. */
  get = async <T>(path: string, params: Params = {}, timeout = REACH_TIMEOUT_MS * 4): Promise<T> => {
    const response = await this.#request(path, params, 'GET', timeout);
    return response.json();
  };

  /** Runs an action, or reads data the PC wraps with the outcome; failures throw a LinkError. */
  call = async <T>(method: 'GET' | 'POST', path: string, params: Params = {}): Promise<T> => {
    const answer: Answer<T> = await (await this.#request(path, params, method, REACH_TIMEOUT_MS * 8)).json();
    if (!answer.ok) throw new LinkError(answer.error);
    return answer.value;
  };

  async #request(path: string, params: Params, method: 'GET' | 'POST', timeout: number) {
    if (!this.#base || !this.pairing) throw new LinkError({ code: 'clientClosed' });
    const response = await fetch(address(this.#base, path, this.pairing.token, params), { method, signal: AbortSignal.timeout(timeout) });
    if (response.status === FORBIDDEN) {
      this.#stop();
      this.status = 'codeChanged';
    }
    if (!response.ok) throw new LinkError({ code: 'noData' });
    return response;
  }

  async #follow(session: number) {
    let version: number | null = null;
    while (session === this.#session) {
      try {
        const answer: { version: number; state: EngineState } = await this.get('/api/state', { after: version }, STATE_TIMEOUT_MS);
        if (session !== this.#session) return;
        version = answer.version;
        this.state = answer.state;
      } catch {
        if (session === this.#session && this.status !== 'codeChanged') this.#lost(session);
        return;
      }
    }
  }

  #lost(session: number) {
    this.status = 'offline';
    this.#retry = setTimeout(() => session === this.#session && this.connect(), RETRY_MS);
  }

  /** Ends the current connection and returns the id of the next one. */
  #stop() {
    clearTimeout(this.#retry);
    this.#base = null;
    return ++this.#session;
  }
}

export const link = new Link();
