use bytes::Bytes;

use crate::error::EngineError;

const MAGIC: [u8; 2] = *b"OX";
const FORMAT_VERSION: u8 = 1;
const FLAG_ZSTD: u8 = 0b0000_0001;

/// Cached HTTP response entry stored in `redb` or the RAM LRU fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub status: u16,
    pub stored_at_unix: u64,
    pub ttl_secs: u32,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub body: Bytes,
}

impl CacheEntry {
    /// Return whether this cached entry is still within its fresh TTL at `now_unix`.
    pub fn is_fresh(&self, now_unix: u64) -> bool {
        now_unix.saturating_sub(self.stored_at_unix) < u64::from(self.ttl_secs)
    }

    /// Return whether this cached entry is fresh against both its stored TTL and `max_ttl_secs`.
    pub fn is_fresh_within(&self, now_unix: u64, max_ttl_secs: u32) -> bool {
        let effective_ttl = self.ttl_secs.min(max_ttl_secs);
        now_unix.saturating_sub(self.stored_at_unix) < u64::from(effective_ttl)
    }
}

/// Encode a [`CacheEntry`] into a compact binary envelope, compressing large bodies with `zstd`.
///
/// Compression is applied only when `entry.body.len() >= zstd_min_bytes` and the
/// compressed output is strictly smaller than the raw payload.
pub fn encode_entry(
    entry: &CacheEntry,
    zstd_min_bytes: usize,
    zstd_level: i32,
) -> Result<Vec<u8>, EngineError> {
    let etag_bytes = entry.etag.as_deref().unwrap_or("").as_bytes();
    let last_mod_bytes = entry.last_modified.as_deref().unwrap_or("").as_bytes();

    let etag_len = u16::try_from(etag_bytes.len())
        .map_err(|_| EngineError::Cache("cached ETag header exceeds 64 KiB".to_owned()))?;
    let last_mod_len = u16::try_from(last_mod_bytes.len())
        .map_err(|_| EngineError::Cache("cached Last-Modified header exceeds 64 KiB".to_owned()))?;

    let (flags, compressed_body): (u8, Option<Vec<u8>>) = if entry.body.len() >= zstd_min_bytes {
        let compressed = zstd::bulk::compress(&entry.body, zstd_level)
            .map_err(|err| EngineError::Cache(format!("zstd compression failed: {err}")))?;
        if compressed.len() < entry.body.len() {
            (FLAG_ZSTD, Some(compressed))
        } else {
            (0, None)
        }
    } else {
        (0, None)
    };

    let payload = compressed_body.as_deref().unwrap_or(&entry.body);
    let header_len = 2 + 1 + 1 + 2 + 8 + 4 + 2 + etag_bytes.len() + 2 + last_mod_bytes.len();
    let mut out = Vec::with_capacity(header_len + payload.len());

    out.extend_from_slice(&MAGIC);
    out.push(FORMAT_VERSION);
    out.push(flags);
    out.extend_from_slice(&entry.status.to_be_bytes());
    out.extend_from_slice(&entry.stored_at_unix.to_be_bytes());
    out.extend_from_slice(&entry.ttl_secs.to_be_bytes());
    out.extend_from_slice(&etag_len.to_be_bytes());
    out.extend_from_slice(etag_bytes);
    out.extend_from_slice(&last_mod_len.to_be_bytes());
    out.extend_from_slice(last_mod_bytes);
    out.extend_from_slice(payload);

    Ok(out)
}

/// Decode a binary cache envelope produced by [`encode_entry`].
pub fn decode_entry(raw: &[u8]) -> Result<CacheEntry, EngineError> {
    let mut cursor = ByteCursor::new(raw);

    let magic = cursor.read_exact(2)?;
    if magic != MAGIC {
        return Err(EngineError::Cache(
            "invalid cache entry magic header".to_owned(),
        ));
    }

    let version = cursor.read_u8()?;
    if version != FORMAT_VERSION {
        return Err(EngineError::Cache(format!(
            "unsupported cache entry version {version}"
        )));
    }

    let flags = cursor.read_u8()?;
    let status = cursor.read_u16_be()?;
    let stored_at_unix = cursor.read_u64_be()?;
    let ttl_secs = cursor.read_u32_be()?;

    let etag_len = usize::from(cursor.read_u16_be()?);
    let etag_raw = cursor.read_exact(etag_len)?;
    let etag = decode_optional_header(etag_raw, "ETag")?;

    let last_mod_len = usize::from(cursor.read_u16_be()?);
    let last_mod_raw = cursor.read_exact(last_mod_len)?;
    let last_modified = decode_optional_header(last_mod_raw, "Last-Modified")?;

    let payload = cursor.rest();
    let body = if flags & FLAG_ZSTD != 0 {
        let decompressed = zstd::stream::decode_all(payload)
            .map_err(|err| EngineError::Cache(format!("zstd decompression failed: {err}")))?;
        Bytes::from(decompressed)
    } else {
        Bytes::copy_from_slice(payload)
    };

    Ok(CacheEntry {
        status,
        stored_at_unix,
        ttl_secs,
        etag,
        last_modified,
        body,
    })
}

fn decode_optional_header(bytes: &[u8], label: &str) -> Result<Option<String>, EngineError> {
    if bytes.is_empty() {
        return Ok(None);
    }

    let s = std::str::from_utf8(bytes)
        .map_err(|err| EngineError::Cache(format!("invalid UTF-8 in cached {label}: {err}")))?;
    Ok(Some(s.to_owned()))
}

struct ByteCursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], EngineError> {
        let end = self
            .pos
            .checked_add(len)
            .filter(|&e| e <= self.buf.len())
            .ok_or_else(|| EngineError::Cache("truncated cache entry header".to_owned()))?;
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, EngineError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16_be(&mut self) -> Result<u16, EngineError> {
        let b = self.read_exact(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn read_u32_be(&mut self) -> Result<u32, EngineError> {
        let b = self.read_exact(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_u64_be(&mut self) -> Result<u64, EngineError> {
        let b = self.read_exact(8)?;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn rest(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_compression_below_threshold_and_compresses_above() {
        let small = CacheEntry {
            status: 200,
            stored_at_unix: 1_700_000_000,
            ttl_secs: 60,
            etag: Some("\"abc\"".to_owned()),
            last_modified: None,
            body: Bytes::from_static(br#"{"id":"AANobbMI"}"#),
        };
        let encoded_small = encode_entry(&small, 512, 3).unwrap();
        assert_eq!(encoded_small[3] & FLAG_ZSTD, 0);
        assert_eq!(decode_entry(&encoded_small).unwrap(), small);

        let repetitive_json = format!(
            "{{\"hits\":[{}]}}",
            vec![r#"{"id":"AANobbMI","slug":"sodium","project_types":["mod"]}"#; 40].join(",")
        );
        let large = CacheEntry {
            status: 200,
            stored_at_unix: 1_700_000_100,
            ttl_secs: 900,
            etag: Some("\"v1\"".to_owned()),
            last_modified: Some("Fri, 20 Nov 2026 03:17:25 GMT".to_owned()),
            body: Bytes::from(repetitive_json),
        };
        let encoded_large = encode_entry(&large, 512, 3).unwrap();
        assert_ne!(encoded_large[3] & FLAG_ZSTD, 0);
        assert!(encoded_large.len() < large.body.len() / 2);
        assert_eq!(decode_entry(&encoded_large).unwrap(), large);
    }

    #[test]
    fn rejects_corrupted_or_truncated_envelopes() {
        assert!(matches!(decode_entry(b"BAD"), Err(EngineError::Cache(_))));
        assert!(matches!(
            decode_entry(b"OX\x02\x00"),
            Err(EngineError::Cache(_))
        ));
        assert!(matches!(
            decode_entry(b"OX\x01\x00"),
            Err(EngineError::Cache(_))
        ));

        // Valid header claiming zstd compression, followed by garbage payload.
        let mut bad_zstd = encode_entry(
            &CacheEntry {
                status: 200,
                stored_at_unix: 10,
                ttl_secs: 10,
                etag: None,
                last_modified: None,
                body: Bytes::from(vec![b'a'; 1024]),
            },
            512,
            3,
        )
        .unwrap();
        let last = bad_zstd.len() - 1;
        bad_zstd[last] ^= 0xFF;
        bad_zstd[last - 1] ^= 0xFF;
        assert!(matches!(
            decode_entry(&bad_zstd),
            Err(EngineError::Cache(_))
        ));
    }

    #[test]
    fn evaluates_ttl_freshness_without_overflow() {
        let entry = CacheEntry {
            status: 200,
            stored_at_unix: 1_000,
            ttl_secs: 120,
            etag: None,
            last_modified: None,
            body: Bytes::new(),
        };
        assert!(entry.is_fresh(1_000));
        assert!(entry.is_fresh(1_119));
        assert!(!entry.is_fresh(1_120));
        assert!(entry.is_fresh(900));
    }
}
