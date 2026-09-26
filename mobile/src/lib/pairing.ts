const PAIRING_PROTOCOL = 'xyra:';
const PAIRING_HOST = 'pair';

export interface Pairing {
  hosts: string[];
  port: string;
  token: string;
}

export type Params = Record<string, string | number | boolean | null | undefined>;

/** Reads a code like xyra://pair?hosts=a,b&port=47811&token=…; null when it is not a Xyra code. */
export function parsePairing(raw: string | null): Pairing | null {
  try {
    const url = new URL(raw ?? '');
    const [hosts, port, token] = ['hosts', 'port', 'token'].map((name) => url.searchParams.get(name));
    if (url.protocol !== PAIRING_PROTOCOL || url.hostname !== PAIRING_HOST || !hosts || !port || !token) return null;
    return { hosts: hosts.split(','), port, token };
  } catch {
    return null;
  }
}

export function address(base: string, path: string, token: string, params: Params = {}) {
  const url = new URL(path, base);
  url.searchParams.set('t', token);
  for (const [name, value] of Object.entries(params)) if (value !== null && value !== undefined) url.searchParams.set(name, String(value));
  return url.toString();
}
