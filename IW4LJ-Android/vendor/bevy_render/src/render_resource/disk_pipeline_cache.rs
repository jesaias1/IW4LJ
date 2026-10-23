//! A Vulkan pipeline cache kept on disk (an IW4LJ patch to Bevy).
//!
//! Compiling a map's graphics pipelines takes minutes on a phone GPU and is
//! repeated on every launch. With a driver pipeline cache saved between runs the
//! second launch reuses the compiled code. The cache is only used when the
//! `IW4L_PIPELINE_CACHE` environment variable names a file and the adapter
//! supports `Features::PIPELINE_CACHE`.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

struct Disk {
    cache: wgpu::PipelineCache,
    path: PathBuf,
    saved_len: Mutex<usize>,
}

static DISK: OnceLock<Disk> = OnceLock::new();

/// The cache to hand to every pipeline created, if there is one.
pub fn disk_pipeline_cache() -> Option<&'static wgpu::PipelineCache> {
    DISK.get().map(|disk| &disk.cache)
}

/// The file named by `IW4L_PIPELINE_CACHE`, if any.
pub fn disk_pipeline_cache_path() -> Option<PathBuf> {
    let value = std::env::var_os("IW4L_PIPELINE_CACHE")?;
    (!value.is_empty()).then(|| PathBuf::from(value))
}

/// Create the cache from the file's bytes. Called once, right after the device.
pub(crate) fn init_disk_pipeline_cache(device: &wgpu::Device) {
    if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
        return;
    }
    let Some(path) = disk_pipeline_cache_path() else {
        return;
    };
    let data = std::fs::read(&path).ok();
    // SAFETY: the bytes came from `PipelineCache::get_data`, and with `fallback`
    // wgpu discards them when they do not match this driver.
    let cache = unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("iw4l pipeline cache"),
            data: data.as_deref(),
            fallback: true,
        })
    };
    let saved_len = data.map_or(0, |data| data.len());
    bevy_log::info!(
        "pipeline cache: {} ({} bytes loaded)",
        path.display(),
        saved_len
    );
    let _ = DISK.set(Disk {
        cache,
        path,
        saved_len: Mutex::new(saved_len),
    });
}

/// Write the cache to its file if it has grown. Returns the size written.
pub fn save_disk_pipeline_cache() -> Option<usize> {
    let disk = DISK.get()?;
    let data = disk.cache.get_data()?;
    let mut saved = disk.saved_len.lock().ok()?;
    if data.len() <= *saved {
        return None;
    }
    let temp = disk.path.with_extension("tmp");
    std::fs::write(&temp, &data).ok()?;
    std::fs::rename(&temp, &disk.path).ok()?;
    *saved = data.len();
    Some(data.len())
}
