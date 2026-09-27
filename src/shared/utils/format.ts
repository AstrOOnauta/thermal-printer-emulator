const UNITS = ['B', 'KB', 'MB'] as const;

/** `512 B`, `1.5 KB`, `16 MB`, with the locale's decimal separator. */
export function formatBytes(bytes: number, locale: string): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const number = new Intl.NumberFormat(locale, {
    maximumFractionDigits: unit === 0 ? 0 : 1,
  }).format(value);
  return `${number} ${UNITS[unit]}`;
}

/** The address part of `ip:port` or `[ipv6]:port`. The client's port is noise. */
export function peerHost(peer: string): string {
  return peer.replace(/:\d+$/, '').replace(/^\[(.*)\]$/, '$1');
}
