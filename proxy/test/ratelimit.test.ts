import { describe, expect, it } from 'vitest';
import { SlidingWindow } from '../src/ratelimit';

describe('SlidingWindow', () => {
  it('allows up to the limit and then blocks', () => {
    const w = new SlidingWindow(3, 1000);
    expect(w.allow('a', 0)).toBe(true);
    expect(w.allow('a', 10)).toBe(true);
    expect(w.allow('a', 20)).toBe(true);
    expect(w.allow('a', 30)).toBe(false);
  });

  it('lets requests through again once they age out of the window', () => {
    const w = new SlidingWindow(2, 1000);
    expect(w.allow('a', 0)).toBe(true);
    expect(w.allow('a', 100)).toBe(true);
    expect(w.allow('a', 200)).toBe(false);
    // İlk iki damga 1000 ms penceresinin dışına çıktı.
    expect(w.allow('a', 1101)).toBe(true);
  });

  it('keys are independent', () => {
    const w = new SlidingWindow(1, 1000);
    expect(w.allow('a', 0)).toBe(true);
    expect(w.allow('a', 1)).toBe(false);
    expect(w.allow('b', 1)).toBe(true);
  });

  it('does not grow without bound', () => {
    const w = new SlidingWindow(1, 10);
    for (let i = 0; i < 5000; i += 1) {
      w.allow(`key-${i}`, i);
    }
    // Süpürme 1024 anahtardan sonra devreye girer; pencere 10 ms olduğu için
    // eski anahtarların ezici çoğunluğu düşmüş olmalı.
    expect(w.trackedKeys).toBeLessThan(2048);
  });
});
