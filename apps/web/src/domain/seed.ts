/** A small seam so tests can make automatic seed generation deterministic. */
export type SeedRandomValues = (bytes: Uint8Array) => void;
export type SeedClock = () => number;

export const SEED_ALPHABET = '123456789abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ';
export const AUTOMATIC_SEED_LENGTH = 4;
const RANDOM_BYTE_LIMIT = Math.floor(256 / SEED_ALPHABET.length) * SEED_ALPHABET.length;

let fallbackCounter = 0;

function browserRandomValues(): SeedRandomValues | undefined {
  if (typeof globalThis === 'undefined' || !globalThis.crypto?.getRandomValues) return undefined;
  return (bytes) => {
    globalThis.crypto.getRandomValues(bytes);
  };
}

function fallbackSeed(): string {
  fallbackCounter += 1;
  let value = fallbackCounter;
  let seed = '';
  for (let index = 0; index < AUTOMATIC_SEED_LENGTH; index += 1) {
    seed = `${SEED_ALPHABET[value % SEED_ALPHABET.length]}${seed}`;
    value = Math.floor(value / SEED_ALPHABET.length);
  }
  return seed;
}

function randomSeed(randomValues: SeedRandomValues): string | undefined {
  const bytes = new Uint8Array(16);
  const characters: string[] = [];
  try {
    // Reject the tail of the byte range so `% alphabet.length` is unbiased.
    // Sixteen bytes is ample for four characters in a real random source;
    // bounded attempts prevent a pathological injected source from looping.
    for (let attempt = 0; attempt < 16 && characters.length < AUTOMATIC_SEED_LENGTH; attempt += 1) {
      randomValues(bytes);
      for (const byte of bytes) {
        if (byte >= RANDOM_BYTE_LIMIT) continue;
        characters.push(SEED_ALPHABET[byte % SEED_ALPHABET.length]!);
        if (characters.length === AUTOMATIC_SEED_LENGTH) break;
      }
    }
  } catch {
    return undefined;
  }
  return characters.length === AUTOMATIC_SEED_LENGTH ? characters.join('') : undefined;
}

/**
 * Generate a four-character seed for a blank q1 seed field. Web Crypto is
 * preferred; the counter fallback uses the same alphabet and distinguishes
 * consecutive calls.
 */
export function generateAutomaticSeed(
  randomValues: SeedRandomValues | undefined = browserRandomValues(),
  _now: SeedClock = Date.now,
): string {
  // Keep the clock argument injectable for API stability and future fallback
  // entropy, while the bounded counter is sufficient to distinguish calls.
  void _now;
  return randomValues ? (randomSeed(randomValues) ?? fallbackSeed()) : fallbackSeed();
}
