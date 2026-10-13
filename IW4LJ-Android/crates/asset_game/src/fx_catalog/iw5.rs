use super::*;

pub(super) trait FxRead {
    fn is_x64(&self) -> bool;
    fn elem_stride(&self) -> usize;
    fn layout(&self, x86: usize, x64: usize) -> usize {
        if self.is_x64() { x64 } else { x86 }
    }
    fn slice_at(&self, ptr: Ptr, offset: usize, len: usize) -> std::result::Result<&[u8], ()>;
    fn ptr_at(&self, ptr: Ptr, offset: usize) -> std::result::Result<ZonePtr, ()>;
    fn i32_at(&self, ptr: Ptr, offset: usize) -> std::result::Result<i32, ()>;
    fn cstr(&self, ptr: Ptr) -> std::result::Result<&str, ()>;
    fn resolve_alias(&self, ptr: Ptr) -> Ptr;
}

impl FxRead for ZoneStream<'_> {
    fn is_x64(&self) -> bool {
        self.pointer_bytes() == 8
    }
    fn elem_stride(&self) -> usize {
        self.layout(sz::FX_ELEM_DEF, 288)
    }
    fn slice_at(&self, p: Ptr, offset: usize, len: usize) -> std::result::Result<&[u8], ()> {
        self.slice_at(p, offset, len).map_err(|_| ())
    }
    fn ptr_at(&self, p: Ptr, offset: usize) -> std::result::Result<ZonePtr, ()> {
        self.ptr_at(p, offset).map_err(|_| ())
    }
    fn i32_at(&self, p: Ptr, offset: usize) -> std::result::Result<i32, ()> {
        self.i32_at(p, offset).map_err(|_| ())
    }
    fn cstr(&self, p: Ptr) -> std::result::Result<&str, ()> {
        self.cstr(p).map_err(|_| ())
    }
    fn resolve_alias(&self, p: Ptr) -> Ptr {
        self.resolve_alias(p)
    }
}

fn native(p: Ptr) -> fastfile_iw5::Ptr {
    fastfile_iw5::Ptr {
        block: p.block,
        offset: p.offset,
    }
}
pub(super) fn shared(p: fastfile_iw5::Ptr) -> Ptr {
    Ptr {
        block: p.block,
        offset: p.offset,
    }
}

impl FxRead for fastfile_iw5::ZoneStream<'_> {
    fn is_x64(&self) -> bool {
        self.pointer_bytes() == 8
    }
    fn elem_stride(&self) -> usize {
        self.layout(fastfile_iw5::size::FX_ELEM_DEF, 288)
    }
    fn slice_at(&self, p: Ptr, offset: usize, len: usize) -> std::result::Result<&[u8], ()> {
        self.slice_at(native(p), offset, len).map_err(|_| ())
    }
    fn ptr_at(&self, p: Ptr, offset: usize) -> std::result::Result<ZonePtr, ()> {
        self.ptr_at(native(p), offset)
            .map(|p| match p {
                fastfile_iw5::ZonePtr::Null => ZonePtr::Null,
                fastfile_iw5::ZonePtr::Following => ZonePtr::Following,
                fastfile_iw5::ZonePtr::Insert => ZonePtr::Insert,
                fastfile_iw5::ZonePtr::Offset(p) => ZonePtr::Offset(shared(p)),
            })
            .map_err(|_| ())
    }
    fn i32_at(&self, p: Ptr, offset: usize) -> std::result::Result<i32, ()> {
        self.i32_at(native(p), offset).map_err(|_| ())
    }
    fn cstr(&self, p: Ptr) -> std::result::Result<&str, ()> {
        self.cstr(native(p)).map_err(|_| ())
    }
    fn resolve_alias(&self, p: Ptr) -> Ptr {
        shared(self.resolve_alias(native(p)))
    }
}

impl FxCatalog {
    pub fn capture_iw5(
        &mut self,
        s: &fastfile_iw5::ZoneStream<'_>,
        geometry: fastfile_iw5::FxEffectDefGeometry,
        materials: &MaterialCatalog,
    ) -> fastfile_iw5::Result<()> {
        let Some(name) = geometry
            .name
            .and_then(|p| s.cstr(p).ok())
            .filter(|name| !name.is_empty())
        else {
            self.capture_gaps += 1;
            return Ok(());
        };
        let view = FxEffectDefView {
            flags: s.i32_at(geometry.header, s.layout(4, 8))?,
            msec_looping_life: s.i32_at(geometry.header, s.layout(12, 16))?,
            looping_count: geometry.looping_count,
            one_shot_count: geometry.one_shot_count,
            emission_count: geometry.emission_count,
        };
        let mut elems = Vec::with_capacity(geometry.elem_def_count);
        if let Some(arr) = geometry.elem_defs {
            for i in 0..geometry.elem_def_count {
                let ptr = arr.at(i * s.elem_stride());
                let Some(elem) = capture_elem(s, shared(ptr), materials, &HashMap::new()) else {
                    self.capture_gaps += 1;
                    return Ok(());
                };
                elems.push(elem);
            }
        } else if geometry.elem_def_count > 0 {
            self.capture_gaps += 1;
            return Ok(());
        }
        self.last_captured = Some(name.to_owned());
        let namespace = self
            .capture_ns
            .expect("asset capture requires an explicit family");
        self.insert_owned(OwnedFxEffectDef {
            namespace,
            name: name.to_owned(),
            view,
            header_raw: leftover_pack_iw4_effect_header(&view),
            elems,
        });
        Ok(())
    }
}
