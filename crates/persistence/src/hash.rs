//! SHA-256 content hashes (engine-spec §8.11 step 4 "record the sha256"; §14.3 "the manifest hash
//! verified on read"; DR-C15; owner ruling R2).
//!
//! [`Sha256Hex`] is a digest. Its text form is exactly 64 lowercase hexadecimal characters, both
//! when displayed and when parsed, so a hash can only ever name a path made of `0-9a-f` (the
//! artifact store derives every path from it: engine-spec §8.11; work package P1-02 D16).

use std::fmt;
use std::io;
use std::str::FromStr;

use sha2::{Digest, Sha256};

/// The lowercase hexadecimal digits.
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// A SHA-256 digest (32 bytes), written as 64 lowercase hexadecimal characters.
///
/// Ordered by its bytes, which is also the order of its text form. Never null: a value always
/// holds a full digest.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sha256Hex([u8; 32]);

impl Sha256Hex {
    /// The length of the text form: 64 characters.
    pub const HEX_LEN: usize = 64;

    /// Wraps 32 digest bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The SHA-256 digest of `bytes`.
    pub fn digest(bytes: &[u8]) -> Self {
        let mut hasher = Sha256Hasher::new();
        hasher.update(bytes);
        hasher.finalize().0
    }

    /// The 64-character lowercase hexadecimal text form.
    pub fn to_hex(&self) -> String {
        let mut text = String::with_capacity(Self::HEX_LEN);
        for byte in self.0 {
            text.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
            text.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
        }
        text
    }

    /// The first two hexadecimal characters: the shard directory of the artifact store
    /// (`artifacts/sha256/<shard>/<hex>`; work package P1-02 D16).
    pub fn shard(&self) -> String {
        let byte = self.0[0];
        let mut text = String::with_capacity(2);
        text.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
        text
    }
}

impl fmt::Display for Sha256Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for Sha256Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256Hex({})", self.to_hex())
    }
}

/// Why a text is not a [`Sha256Hex`]. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseSha256Error {
    /// The text is not 64 bytes long.
    Length {
        /// The length found, in bytes.
        found: usize,
    },
    /// A byte is not one of `0-9a-f` (upper case is refused, never lower-cased).
    NotLowercaseHex {
        /// The byte offset of the first offending byte.
        offset: usize,
    },
}

impl fmt::Display for ParseSha256Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseSha256Error::Length { found } => write!(
                f,
                "a sha256 is 64 lowercase hexadecimal characters; found {found} bytes"
            ),
            ParseSha256Error::NotLowercaseHex { offset } => write!(
                f,
                "a sha256 is 64 lowercase hexadecimal characters; byte {offset} is not 0-9a-f"
            ),
        }
    }
}

impl std::error::Error for ParseSha256Error {}

/// The value of one lowercase hexadecimal digit.
const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

impl FromStr for Sha256Hex {
    type Err = ParseSha256Error;

    /// Parses exactly 64 lowercase hexadecimal characters. Nothing is normalized: upper case, a
    /// prefix, whitespace or a short hash is refused.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bytes = text.as_bytes();
        if bytes.len() != Self::HEX_LEN {
            return Err(ParseSha256Error::Length { found: bytes.len() });
        }
        let mut digest = [0u8; 32];
        for (index, slot) in digest.iter_mut().enumerate() {
            let high = hex_value(bytes[2 * index])
                .ok_or(ParseSha256Error::NotLowercaseHex { offset: 2 * index })?;
            let low = hex_value(bytes[2 * index + 1]).ok_or(ParseSha256Error::NotLowercaseHex {
                offset: 2 * index + 1,
            })?;
            *slot = (high << 4) | low;
        }
        Ok(Self(digest))
    }
}

/// A streaming SHA-256 hasher that also counts the bytes it has seen.
///
/// It implements [`io::Write`], so a reader can be hashed with [`io::copy`]; its writes never
/// fail.
#[derive(Clone, Default)]
pub struct Sha256Hasher {
    inner: Sha256,
    len: u64,
}

impl Sha256Hasher {
    /// A hasher that has seen no bytes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds `bytes` to the hash.
    pub fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
        // usize → u64 is lossless on every supported target; saturate rather than wrap.
        let added = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        self.len = self.len.saturating_add(added);
    }

    /// The number of bytes fed so far.
    pub const fn len(&self) -> u64 {
        self.len
    }

    /// Whether no byte has been fed.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The digest of every byte fed, and how many bytes that was.
    pub fn finalize(self) -> (Sha256Hex, u64) {
        let output = self.inner.finalize();
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&output[..]);
        (Sha256Hex(digest), self.len)
    }
}

impl io::Write for Sha256Hasher {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips_and_is_strict() {
        let hash = Sha256Hex::digest(b"abc");
        let text = hash.to_hex();
        assert_eq!(text.len(), 64);
        assert_eq!(text.parse::<Sha256Hex>(), Ok(hash));
        assert_eq!(hash.shard(), text[..2]);
        assert_eq!(
            text.to_uppercase().parse::<Sha256Hex>(),
            Err(ParseSha256Error::NotLowercaseHex { offset: 0 })
        );
        assert_eq!(
            text[..63].parse::<Sha256Hex>(),
            Err(ParseSha256Error::Length { found: 63 })
        );
    }
}
