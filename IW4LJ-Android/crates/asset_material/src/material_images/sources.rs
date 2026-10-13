use super::decode::{CubemapFaces, DecodedMips, decode_iwi_cubemap};
use super::disk_cache::load_or_decode_mips;
use crate::ImageVariantId;
use asset_iw4::{IWI_V8_HEADER_LEN, IwiHeader};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct IwdIndex(Arc<asset_transport::IwdIndex>);

pub fn cached_iwd_main_dirs() -> Vec<PathBuf> {
    asset_transport::cached_iwd_dirs()
}

impl IwdIndex {
    pub(super) fn from_owned(index: Arc<asset_transport::IwdIndex>) -> Self {
        Self(index)
    }
    pub(super) fn empty() -> Self {
        Self(Arc::new(asset_transport::IwdIndex::default()))
    }
    pub(super) fn archive_count(&self) -> usize {
        self.0.archive_count()
    }
    pub(super) fn variant(&self, name: &str, map_type: u8, usage: u32) -> Option<ImageVariantId> {
        let candidates = self.0.image_candidates(name)?;
        let mut payload = crate::fnv1a64(name.as_bytes());
        for candidate in candidates {
            payload = crate::fnv1a64_more(payload, &candidate.crc32().to_le_bytes());
            payload = crate::fnv1a64_more(payload, &candidate.size().to_le_bytes());
            payload = crate::fnv1a64_more(payload, candidate.entry().as_bytes());
        }
        payload = crate::fnv1a64_more(payload, &[map_type]);
        Some(ImageVariantId { payload, usage })
    }

    pub(super) fn open(directory: &Path) -> Result<Arc<Self>, String> {
        asset_transport::IwdIndex::open(directory).map(|index| Arc::new(Self(index)))
    }

    pub(super) fn is_cached(directory: &Path) -> bool {
        asset_transport::IwdIndex::is_cached(directory)
    }

    pub(super) fn decode(&self, name: &str) -> Option<Result<DecodedMips, String>> {
        let candidates = self.0.image_candidates(name)?;
        let mut first_error = None;
        for candidate in candidates {
            match load_or_decode_mips(candidate) {
                Ok(image) => return Some(Ok(image)),
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        Some(Err(
            first_error.unwrap_or_else(|| "empty IWD candidate list".into())
        ))
    }

    pub(super) fn decode_cubemap_if_skybox(
        &self,
        name: &str,
        map_type: u8,
    ) -> Option<Result<(u32, CubemapFaces), String>> {
        let candidates = self.0.image_candidates(name)?;
        let mut saw_skybox = false;
        let mut first_error = None;
        for candidate in candidates {
            if map_type != 5 {
                match candidate.read_header(IWI_V8_HEADER_LEN) {
                    Ok(header) if IwiHeader::parse(&header).is_ok_and(|h| h.is_skybox()) => {}
                    Ok(_) => continue,
                    Err(error) => {
                        if first_error.is_none() {
                            first_error = Some(error);
                        }
                        continue;
                    }
                }
            }
            match candidate.read() {
                Ok(bytes) => {
                    saw_skybox = true;
                    match decode_iwi_cubemap(&bytes) {
                        Ok(image) => return Some(Ok(image)),
                        Err(error) if first_error.is_none() => first_error = Some(error),
                        Err(_) => {}
                    }
                }
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        saw_skybox.then(|| Err(first_error.unwrap_or_else(|| "empty IWD candidate list".into())))
    }
}
