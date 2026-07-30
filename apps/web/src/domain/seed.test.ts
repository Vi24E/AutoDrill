import { describe, expect, it } from 'vitest';

import { AUTOMATIC_SEED_LENGTH, generateAutomaticSeed, SEED_ALPHABET } from '@/domain/seed';

describe('automatic seed generation', () => {
  it('prefers the injected crypto source and emits four allowed characters', () => {
    const seed = generateAutomaticSeed((bytes) => bytes.fill(0xab), () => 0);
    expect(seed).toHaveLength(AUTOMATIC_SEED_LENGTH);
    expect(seed).toBe(`${SEED_ALPHABET[0xab % SEED_ALPHABET.length]}`.repeat(AUTOMATIC_SEED_LENGTH));
    expect([...seed].every((character) => SEED_ALPHABET.includes(character))).toBe(true);
    expect(seed).not.toMatch(/[IlO]/);
  });

  it('uses rejection sampling and a distinct four-character fallback', () => {
    let calls = 0;
    const rejectionThenAccepted = (bytes: Uint8Array) => {
      calls += 1;
      bytes.fill(calls === 1 ? 0xff : 0);
    };
    expect(generateAutomaticSeed(rejectionThenAccepted, () => 1234)).toBe('1111');
    expect(calls).toBe(2);

    const failingRandom = () => {
      throw new Error('unavailable');
    };
    const first = generateAutomaticSeed(failingRandom, () => 1234);
    const second = generateAutomaticSeed(failingRandom, () => 1234);
    expect(first).not.toBe(second);
    expect(first).toHaveLength(AUTOMATIC_SEED_LENGTH);
    expect(second).toHaveLength(AUTOMATIC_SEED_LENGTH);
    expect([...first, ...second].every((character) => SEED_ALPHABET.includes(character))).toBe(true);
  });
});
