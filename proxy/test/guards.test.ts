import { describe, expect, it } from 'vitest';
import { isAllowedOrigin, parseAllowedOrigins, rejectTarget } from '../src/guards';

describe('rejectTarget', () => {
  it('accepts a plain https url', () => {
    expect(rejectTarget('https://cdn.mbta.com/realtime/Alerts.pb')).toBeNull();
  });

  it('rejects non-https schemes', () => {
    for (const url of [
      'http://cdn.mbta.com/a.pb',
      'file:///etc/passwd',
      'ftp://example.org/a.pb',
      'gopher://example.org/',
    ]) {
      expect(rejectTarget(url), url).toBe('not_https');
    }
  });

  it('rejects localhost in every spelling', () => {
    for (const host of ['localhost', 'LOCALHOST', 'foo.localhost', 'ip6-localhost']) {
      expect(rejectTarget(`https://${host}/a.pb`), host).toBe('localhost_blocked');
    }
    expect(rejectTarget('https://[::1]/a.pb')).toBe('localhost_blocked');
  });

  it('rejects private and reserved IPv4 ranges', () => {
    for (const ip of [
      '10.0.0.1',
      '169.254.169.254', // cloud metadata endpoint
      '172.16.0.1',
      '172.31.255.255',
      '192.168.1.1',
      '0.0.0.0',
      '100.64.0.1', // CGNAT
    ]) {
      expect(rejectTarget(`https://${ip}/a.pb`), ip).toBe('private_address_blocked');
    }
  });

  it('allows public IPv4 literals', () => {
    // 172.32 özel aralığın DIŞINDA; sınırın doğru yerde olduğunu kanıtlar.
    for (const ip of ['8.8.8.8', '172.32.0.1', '11.0.0.1', '100.128.0.1']) {
      expect(rejectTarget(`https://${ip}/a.pb`), ip).toBeNull();
    }
  });

  it('rejects loopback however it is spelled in IPv6', () => {
    // ÖLÇÜLDÜ: WHATWG URL bu adresleri hex'e normalize ediyor
    // (`::ffff:127.0.0.1` -> `::ffff:7f00:1`), yani noktalı gösterimi arayan bir
    // kontrol loopback'i kaçırır. Bunlar gerçek bir atlatma vektörüydü.
    for (const host of [
      '[::ffff:127.0.0.1]',        // IPv4-mapped
      '[0:0:0:0:0:ffff:127.0.0.1]', // aynı adres, uzun yazım
      '[::127.0.0.1]',             // IPv4-compatible
      '[::ffff:7f00:1]',           // normalize edilmiş hâli, doğrudan
    ]) {
      expect(rejectTarget(`https://${host}/a.pb`), host).toBe('localhost_blocked');
    }
  });

  it('rejects private ranges reached through IPv4-mapped IPv6', () => {
    for (const host of ['[::ffff:10.0.0.1]', '[::ffff:169.254.169.254]', '[::ffff:192.168.1.1]']) {
      expect(rejectTarget(`https://${host}/a.pb`), host).toBe('private_address_blocked');
    }
  });

  it('allows a public address written as IPv4-mapped IPv6', () => {
    // Kapının fazla geniş olmadığını gösterir: eşleme biçimi tek başına suç değil.
    expect(rejectTarget('https://[::ffff:8.8.8.8]/a.pb')).toBeNull();
  });

  it('normalises obfuscated IPv4 spellings before checking', () => {
    // ÖLÇÜLDÜ: bu klasik SSRF atlatma biçimlerini WHATWG URL zaten
    // 127.0.0.1'e çeviriyor, dolayısıyla tek biçimi denetlemek yetiyor.
    for (const host of ['2130706433', '0177.0.0.1', '127.1', '0x7f000001']) {
      expect(rejectTarget(`https://${host}/a.pb`), host).toBe('localhost_blocked');
    }
  });

  it('rejects IPv6 link-local and unique-local', () => {
    expect(rejectTarget('https://[fe80::1]/a.pb')).toBe('private_address_blocked');
    expect(rejectTarget('https://[fd00::1]/a.pb')).toBe('private_address_blocked');
  });

  it('rejects credentials embedded in the url', () => {
    expect(rejectTarget('https://user:pass@example.org/a.pb')).toBe('credentials_in_url');
  });

  it('rejects malformed urls', () => {
    expect(rejectTarget('not a url')).toBe('malformed_url');
    expect(rejectTarget('')).toBe('malformed_url');
  });
});

describe('origin handling', () => {
  const allowed = ['https://ttezer.github.io'];

  it('accepts only exact origins', () => {
    expect(isAllowedOrigin('https://ttezer.github.io', allowed)).toBe(true);
    expect(isAllowedOrigin('https://ttezer.github.io.evil.test', allowed)).toBe(false);
    expect(isAllowedOrigin('http://ttezer.github.io', allowed)).toBe(false);
    expect(isAllowedOrigin(null, allowed)).toBe(false);
  });

  it('parses a comma separated list', () => {
    expect(parseAllowedOrigins('https://a.test, https://b.test')).toEqual([
      'https://a.test',
      'https://b.test',
    ]);
    expect(parseAllowedOrigins(undefined)).toEqual([]);
    expect(parseAllowedOrigins('')).toEqual([]);
  });
});
