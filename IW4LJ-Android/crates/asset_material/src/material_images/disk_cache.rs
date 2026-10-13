use super::decode::{DecodedMips, MipStorage, decode_iwi_mips};
use std::sync::atomic::{AtomicU64, Ordering};

const MIP_CACHE_FORMAT: u32 = 3;
const MIP_MAGIC: &[u8; 8] = b"IWL1MIPS";
static MIP_HIT: AtomicU64 = AtomicU64::new(0);
static MIP_MISS: AtomicU64 = AtomicU64::new(0);
static MIP_IO_NS: AtomicU64 = AtomicU64::new(0);

pub fn mip_cache_cost() -> (u64, u64, f64) {
    (
        MIP_HIT.load(Ordering::Relaxed),
        MIP_MISS.load(Ordering::Relaxed),
        MIP_IO_NS.load(Ordering::Relaxed) as f64 / 1.0e6,
    )
}

fn encode_cache(mips: &DecodedMips) -> Vec<u8> {
    let mut out = Vec::with_capacity(28 + 4 * mips.level_sizes().len() + mips.payload().len());
    out.extend_from_slice(MIP_MAGIC);
    out.extend_from_slice(&MIP_CACHE_FORMAT.to_le_bytes());
    out.extend_from_slice(&mips.layout().width.to_le_bytes());
    out.extend_from_slice(&mips.layout().height.to_le_bytes());
    out.push(match mips.layout().storage {
        MipStorage::R32Float => 6,
        MipStorage::Rgba8 => 0,
        MipStorage::Bc1 => 1,
        MipStorage::Bc2 => 2,
        MipStorage::Bc3 => 3,
        MipStorage::Bc5 => 5,
        MipStorage::Etc2Rgb => 7,
        MipStorage::Etc2Rgba => 8,
        MipStorage::R16Float => 9,
    });
    out.extend_from_slice(&[0, 0, 0]);
    out.extend_from_slice(&(mips.level_sizes().len() as u32).to_le_bytes());
    let mut at = 0usize;
    for &size in mips.level_sizes() {
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&mips.payload()[at..at + size as usize]);
        at += size as usize;
    }
    out
}

fn decode_cache(bytes: &[u8]) -> Option<DecodedMips> {
    if bytes.len() < 28 || &bytes[..8] != MIP_MAGIC {
        return None;
    }
    let word = |at: usize| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    if word(8)? != MIP_CACHE_FORMAT {
        return None;
    }
    let width = word(12)?;
    let height = word(16)?;
    let storage = match bytes[20] {
        // Written before Android stored float images as half-float.
        6 if cfg!(target_os = "android") => return None,
        6 => MipStorage::R32Float,
        0 => MipStorage::Rgba8,
        1 => MipStorage::Bc1,
        2 => MipStorage::Bc2,
        3 => MipStorage::Bc3,
        5 => MipStorage::Bc5,
        7 => MipStorage::Etc2Rgb,
        8 => MipStorage::Etc2Rgba,
        9 => MipStorage::R16Float,
        _ => return None,
    };
    let count = word(24)? as usize;
    if count == 0 || count > bytes.len().saturating_sub(28) / 4 {
        return None;
    }
    let mut at = 28usize;
    let mut packed = Vec::with_capacity(bytes.len().saturating_sub(28));
    let mut sizes = Vec::with_capacity(count);
    for _ in 0..count {
        let size = word(at)?;
        at = at.checked_add(4)?;
        let end = at.checked_add(size as usize)?;
        packed.extend_from_slice(bytes.get(at..end)?);
        at = end;
        sizes.push(size);
    }
    if at != bytes.len() {
        return None;
    }
    DecodedMips::from_packed(width, height, storage, packed, sizes)
}

fn read_cached_mips(key: &str, io_at: std::time::Instant) -> Option<DecodedMips> {
    let hit = crate::cache_get("mips", key)?;
    MIP_IO_NS.fetch_add(io_at.elapsed().as_nanos() as u64, Ordering::Relaxed);
    let mips = decode_cache(&hit)?;
    MIP_HIT.fetch_add(1, Ordering::Relaxed);
    Some(mips)
}

fn mip_cache_key(crc: u32, size: u64, entry: &str) -> String {
    let key = format!(
        "{MIP_CACHE_FORMAT:08x}-{crc:08x}-{size:x}-{:016x}",
        crate::fnv1a64(entry.as_bytes())
    );
    // What is cached is what the device was handed, so a quality tier that
    // leaves levels out has entries of its own.
    match super::decode::texture_skip_levels() {
        0 => key,
        skip => format!("{key}-s{skip}"),
    }
}

pub(super) fn load_or_decode_mips(
    candidate: &asset_transport::IwdFile,
) -> Result<DecodedMips, String> {
    let key = mip_cache_key(candidate.crc32(), candidate.size(), candidate.entry());
    let io_at = std::time::Instant::now();
    if let Some(mips) = read_cached_mips(&key, io_at) {
        return Ok(mips);
    }

    let _flight = crate::cache_flight("mips", &key);
    if let Some(mips) = read_cached_mips(&key, std::time::Instant::now()) {
        return Ok(mips);
    }
    let bytes = candidate.read()?;
    let mips = decode_iwi_mips(&bytes)?;
    let blob = encode_cache(&mips);
    let store_at = std::time::Instant::now();
    if let Err(error) = crate::cache_put("mips", &key, &blob) {
        diag::warn!(Zone, "mip cache store {key}: {error}");
    }
    MIP_IO_NS.fetch_add(store_at.elapsed().as_nanos() as u64, Ordering::Relaxed);
    MIP_MISS.fetch_add(1, Ordering::Relaxed);
    Ok(mips)
}
