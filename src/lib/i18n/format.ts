const BYTES_PER_KILOBYTE = 1024;

export function formatter(language: string) {
  const numbers = (options: Intl.NumberFormatOptions) => new Intl.NumberFormat(language, options);
  const dates = new Intl.DateTimeFormat(language, { dateStyle: 'medium', timeStyle: 'short' });
  return {
    number: (value: number, digits = 0) => numbers({ minimumFractionDigits: digits, maximumFractionDigits: digits }).format(value),
    percent: (value: number, digits = 0) => numbers({ style: 'percent', minimumFractionDigits: digits, maximumFractionDigits: digits }).format(value / 100),
    compact: (value: number) => numbers({ notation: 'compact' }).format(value),
    signed: (value: number) => numbers({ signDisplay: 'exceptZero' }).format(value),
    seconds: (value: number) => numbers({ style: 'unit', unit: 'second', unitDisplay: 'narrow' }).format(value),
    kilobytes: (bytes: number) => numbers({ style: 'unit', unit: 'kilobyte', maximumFractionDigits: 0 }).format(Math.max(1, bytes / BYTES_PER_KILOBYTE)),
    date: (iso: string) => dates.format(new Date(iso)),
    /** A length of time as minutes and seconds, like 18:43. */
    clock: (seconds: number) => `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`,
  };
}

export type Formatter = ReturnType<typeof formatter>;
