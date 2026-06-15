use super::len_u64;
use std::sync::LazyLock;
static K_LIST: LazyLock<[u32; 64]> = LazyLock::new(gen_k);
static H_LIST: LazyLock<[u32; 8]> = LazyLock::new(gen_h);
fn is_prime(n: u32) -> bool {
  if n < 2 {
    return false;
  }
  let limit = (n as f64).sqrt() as u32;
  for i in 2..=limit {
    if n.is_multiple_of(i) {
      return false;
    }
  }
  true
}
#[expect(clippy::float_arithmetic)]
fn gen_k() -> [u32; 64] {
  let mut primes = vec![];
  let mut n = 2u32;
  while primes.len() < 64 {
    if is_prime(n) {
      primes.push(n);
    }
    n += 1;
  }
  let mut k_list = [0u32; 64];
  for i in 0..64 {
    let x = (primes[i] as f64).cbrt();
    let frac = x.fract();
    let k_elem = (frac * (1u64 << 32) as f64).floor() as u32;
    k_list[i] = k_elem;
  }
  k_list
}
#[expect(clippy::float_arithmetic)]
fn gen_h() -> [u32; 8] {
  let mut primes = vec![];
  let mut n = 2u32;
  while primes.len() < 8 {
    if is_prime(n) {
      primes.push(n);
    }
    n += 1;
  }
  let mut h_list = [0u32; 8];
  for i in 0..8 {
    let x = (primes[i] as f64).sqrt();
    let frac = x.fract();
    h_list[i] = (frac * (1u64 << 32) as f64).floor() as u32;
  }
  h_list
}
#[inline]
fn ch(x: u32, y: u32, z: u32) -> u32 {
  (x & y) ^ (!x & z)
}
#[inline]
fn maj(x: u32, y: u32, z: u32) -> u32 {
  (x & y) ^ (x & z) ^ (y & z)
}
#[inline]
fn big_sigma0(x: u32) -> u32 {
  x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}
#[inline]
fn big_sigma1(x: u32) -> u32 {
  x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}
#[inline]
fn small_sigma0(x: u32) -> u32 {
  x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}
#[inline]
fn small_sigma1(x: u32) -> u32 {
  x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}
pub(crate) fn sha256(data: &[u8]) -> [u8; 32] {
  fn w_add_all(values: &[u32]) -> u32 {
    values.iter().fold(0u32, |acc, &val| acc.wrapping_add(val))
  }
  let mut h1 = *H_LIST;
  let bit_len = len_u64(data) * 8;
  let mut msg = data.to_vec();
  msg.push(0x80);
  msg.resize((msg.len() + 8).next_multiple_of(64) - 8, 0);
  msg.extend_from_slice(&bit_len.to_be_bytes());
  for chunk in msg.chunks(64) {
    let mut w = [0u32; 64];
    for i in 0..16 {
      w[i] =
        u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
    }
    for i in 16..64 {
      w[i] = w_add_all(&[w[i - 16], small_sigma0(w[i - 15]), w[i - 7], small_sigma1(w[i - 2])]);
    }
    let mut h2 = h1;
    for i in 0..64 {
      let tmp1 = w_add_all(&[h2[7], big_sigma1(h2[4]), ch(h2[4], h2[5], h2[6]), K_LIST[i], w[i]]);
      let tmp2 = w_add_all(&[big_sigma0(h2[0]), maj(h2[0], h2[1], h2[2])]);
      h2.copy_within(0..7, 1);
      h2[4] = h2[4].wrapping_add(tmp1);
      h2[0] = tmp1.wrapping_add(tmp2);
    }
    for (hi, vi) in h1.iter_mut().zip(h2.iter().copied()) {
      *hi = hi.wrapping_add(vi);
    }
  }
  let mut out = [0u8; 32];
  for i in 0..8 {
    out[i * 4..i * 4 + 4].copy_from_slice(&h1[i].to_be_bytes());
  }
  out
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn sha_256() {
    assert_eq!(
      sha256(b"abc"),
      *b"\xBA\x78\x16\xBF\x8F\x01\xCF\xEA\x41\x41\x40\xDE\x5D\xAE\x22\x23\
         \xB0\x03\x61\xA3\x96\x17\x7A\x9C\xB4\x10\xFF\x61\xF2\x00\x15\xAD"
    );
  }
}
