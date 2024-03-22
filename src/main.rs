#![no_main]
#![no_std]
#![deny(warnings)]

extern crate alloc;

mod config;
mod boot_info;

use core::slice;


use boot_info::{BootInfo, GraphicsInfo};
use elf::endian::AnyEndian;
use uefi::{
    prelude::*,
    proto::{
        console::gop::GraphicsOutput,
        media::file::{Directory, File, FileAttribute, FileHandle, FileInfo, FileMode},
    },
    table::boot::{AllocateType, MemoryMapSize, MemoryType, ScopedProtocol},
    CStr16, Error as EfiError,
    Result as EfiResult,
};
use uefi_services::{print, println};

#[allow(non_snake_case)]
#[no_mangle]
pub fn LdrConstructBootInfo(bt: &BootServices) -> Result<boot_info::BootInfo, EfiError> {
    let MemoryMapSize { map_size: memmap_size, entry_size: memdesc_size }
        = bt.memory_map_size();

    let memmap = unsafe { slice::from_raw_parts_mut(
        bt.allocate_pool(MemoryType::LOADER_DATA, memmap_size + 0x1000)?,
        memmap_size + 0x1000
    ) };

    _ = bt.memory_map(memmap)?;

    let mut gop = initialize_gop(bt)?;

    let mut fb = gop.frame_buffer();

    let fb_base = fb.as_mut_ptr().clone();
    let fb_size = fb.size().clone();

    drop(fb);

    Ok(BootInfo {
        antboot_version: 1,
        memmap: memmap.as_mut_ptr().cast(),
        memdesc_size,
        memmap_size,
        graphics: GraphicsInfo {
            ty: 1,
            width: gop.current_mode_info().resolution().0,
            height: gop.current_mode_info().resolution().1,
            fb_base,
            fb_size,
            pixels_per_scanline: gop.current_mode_info().stride(),
            pixel_fmt: gop.current_mode_info().pixel_format(),
        }
    })

}

#[allow(non_snake_case)]
#[no_mangle]
pub fn LdrOpenBootVolume(image: Handle) -> EfiResult<Directory> {
    let system_table = uefi_services::system_table();
    let mut sfs = system_table.boot_services().get_image_file_system(image)?;

    sfs.open_volume()
}

#[allow(non_snake_case)]
#[no_mangle]
pub fn LdrOpenSubdirectory(parent: &mut Directory, sub: &CStr16) -> EfiResult<Directory> {
    parent.open(sub, FileMode::Read, FileAttribute::empty())?
        .into_directory()
        .ok_or(EfiError::new(Status::INVALID_PARAMETER, ()))
}

#[entry]
fn main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    let mut root_volume = match LdrOpenBootVolume(image) {
        Ok(v) => v,
        Err(e) => {
            println!("\r\nLdrBootVolume() failed: {}!", e.status());
            system_table
                .boot_services()
                .stall(10_000_000);    
            return e.status();
        }
    };

    let mut _sys_dir = match LdrOpenSubdirectory(&mut root_volume, cstr16!("System")){
        Ok(d) => d,
        Err(e) => {
            println!("\r\nFailed to open 'System': {}!", e.status());
            system_table
                .boot_services()
                .stall(10_000_000);
            root_volume.close();
            return e.status();
        }
    };

    let mut _drv_dir = match LdrOpenSubdirectory(&mut root_volume, cstr16!("Drivers")){
        Ok(d) => d,
        Err(e) => {
            println!("\r\nFailed to open 'Drivers': {}!", e.status());
            system_table
                .boot_services()
                .stall(10_000_000);
            root_volume.close();
            return e.status();
        }
    };

    let _kernel = match load(_sys_dir, cstr16!("AntKrnl.exe"), false) {
        Ok(fil) => fil,
        Err(e) => {
            println!("\r\nFailed to load AntKrnl.exe: {}!", e.status());
            system_table
                .boot_services()
                .stall(10_000_000);    
            return e.status();
        }
    }; 

    println!("{:#?}", LdrConstructBootInfo(system_table.boot_services()));

    let _kernel_elf = match elf::ElfBytes::<AnyEndian>::minimal_parse(_kernel){
        Ok(elf) => elf,
        Err(e) => {
            println!("Failed to parse AntKrnl.exe: {}", e);
            system_table
            .boot_services()
            .stall(10_000_000);

            _ = unsafe { system_table.boot_services().free_pool(_kernel.as_ptr() as *mut u8) };

            return Status::LOAD_ERROR;
        }
    };

    system_table
    .boot_services()
    .stall(10_000_000);    

    Status::SUCCESS
}

pub fn initialize_gop<'a>(boot_services: &'a BootServices) -> Result<ScopedProtocol<'a, GraphicsOutput>, EfiError> {
    let gop_handle = boot_services.get_handle_for_protocol::<GraphicsOutput>()?;
    boot_services.open_protocol_exclusive::<GraphicsOutput>(gop_handle)
}

pub fn load<'a>(mut dir: Directory, path: &CStr16, _toplevel: bool) -> Result<&'a [u8], EfiError> {
    let system_table = uefi_services::system_table();

    print!("Loading {}...", &path);

    let mut handle: FileHandle;
    handle = dir.open(path, FileMode::Read, FileAttribute::empty())?;

    let _info = handle.get_boxed_info::<FileInfo>()?;

    let file_size = _info.file_size();



    let buffer_ptr = system_table.boot_services().allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        (file_size / 0x1000) as usize + 1,
    )? as *mut u8;

    if buffer_ptr.is_null() {
        return Err(EfiError::new(Status::OUT_OF_RESOURCES, ()));
    }

    let buffer = unsafe { slice::from_raw_parts_mut(buffer_ptr, file_size as usize) };

    let mut file = handle
        .into_regular_file()
        .ok_or(EfiError::new(Status::UNSUPPORTED, ()))?;

    if file.read(buffer)? < file_size as usize {
        return Err(EfiError::new(Status::LOAD_ERROR, ()));
    }

    print!("\tSuccess\r\n");

    Ok(buffer)
}
