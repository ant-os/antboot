use uefi::{proto::console::gop::PixelFormat, table::boot::MemoryDescriptor};



#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BootInfo {
    pub antboot_version: u8,
    pub memmap: *mut MemoryDescriptor,
    pub memdesc_size: usize,
    pub memmap_size: usize,
    pub graphics: GraphicsInfo,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GraphicsInfo {
    pub ty: u8, // always 0x1(gop)
    pub width: usize,
    pub height: usize,
    pub pixel_fmt: PixelFormat,
    pub pixels_per_scanline: usize,
    pub fb_base: *mut u8,
    pub fb_size: usize,
}

