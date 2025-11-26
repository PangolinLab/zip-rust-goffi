use libc::{c_char, c_int, c_void, free, malloc};
use std::ffi::{CStr, CString};
use std::io::{Cursor, Read, Write};
use std::ptr;
use std::slice;

use zip::read::ZipArchive;
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

/// Error codes
/// 0 == OK, non-zero == error
const ERR_NULL: c_int = 1;
const ERR_IO: c_int = 2;
const ERR_ZIP: c_int = 3;
const ERR_ALLOC: c_int = 4;

#[no_mangle]
pub extern "C" fn zip_compress(
    data: *const u8,
    len: usize,
    entry_name: *const c_char,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> c_int {
    if data.is_null() || entry_name.is_null() || out_ptr.is_null() || out_len.is_null() {
        return ERR_NULL;
    }
    unsafe {
        let input = slice::from_raw_parts(data, len);
        let cstr = match CStr::from_ptr(entry_name).to_str() {
            Ok(s) => s,
            Err(_) => return ERR_NULL,
        };

        // Build zip archive in memory
        let mut writer = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut writer));
            let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
            if let Err(_) = zip.start_file(cstr, options) {
                return ERR_ZIP;
            }
            if let Err(_) = zip.write_all(input) {
                return ERR_IO;
            }
            if let Err(_) = zip.finish() {
                return ERR_ZIP;
            }
        }
        // writer is Cursor -> inner Vec, but Cursor used mutable ref; get bytes:
        // the Cursor wrapped &mut writer so writer now contains the bytes
        let out_bytes = writer;

        // allocate via malloc so Go/C can free with free_buffer
        let n = out_bytes.len();
        if n == 0 {
            // Still return an allocated zero-length pointer (NULL is acceptable but handle uniformly)
            *out_ptr = ptr::null_mut();
            *out_len = 0usize;
            return 0;
        }
        let mem = malloc(n) as *mut u8;
        if mem.is_null() {
            return ERR_ALLOC;
        }
        ptr::copy_nonoverlapping(out_bytes.as_ptr(), mem, n);
        *out_ptr = mem;
        *out_len = n;
        0
    }
}

#[no_mangle]
pub extern "C" fn zip_decompress_first(
    zip_data: *const u8,
    zip_len: usize,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_name: *mut *mut c_char,
) -> c_int {
    if zip_data.is_null() || out_ptr.is_null() || out_len.is_null() || out_name.is_null() {
        return ERR_NULL;
    }
    unsafe {
        let buf = slice::from_raw_parts(zip_data, zip_len);
        let cursor = Cursor::new(buf);
        let mut archive = match ZipArchive::new(cursor) {
            Ok(a) => a,
            Err(_) => return ERR_ZIP,
        };
        if archive.len() == 0 {
            *out_ptr = ptr::null_mut();
            *out_len = 0;
            *out_name = ptr::null_mut();
            return 0;
        }
        // Extract first file (index 0)
        let mut file = match archive.by_index(0) {
            Ok(f) => f,
            Err(_) => return ERR_ZIP,
        };
        let mut out = Vec::with_capacity(file.size() as usize);
        if let Err(_) = file.read_to_end(&mut out) {
            return ERR_IO;
        }
        // prepare filename (as C string)
        let fname = match CString::new(file.name()) {
            Ok(s) => s,
            Err(_) => return ERR_IO,
        };
        // allocate output buffer
        let n = out.len();
        if n == 0 {
            *out_ptr = ptr::null_mut();
            *out_len = 0usize;
        } else {
            let mem = malloc(n) as *mut u8;
            if mem.is_null() {
                return ERR_ALLOC;
            }
            ptr::copy_nonoverlapping(out.as_ptr(), mem, n);
            *out_ptr = mem;
            *out_len = n;
        }
        // allocate and return file name (malloc + copy)
        let name_bytes = fname.to_bytes_with_nul();
        let name_len = name_bytes.len();
        let name_mem = malloc(name_len) as *mut c_char;
        if name_mem.is_null() {
            // free previously allocated buffer if any
            if !(*out_ptr).is_null() {
                free(*out_ptr as *mut c_void);
                *out_ptr = ptr::null_mut();
                *out_len = 0;
            }
            return ERR_ALLOC;
        }
        ptr::copy_nonoverlapping(name_bytes.as_ptr() as *const c_char, name_mem, name_len);
        *out_name = name_mem;
        0
    }
}

/// Free buffer allocated by this library (both data buffers and name buffers).
#[no_mangle]
pub extern "C" fn zipffi_free_buffer(ptr_buf: *mut c_void) {
    if ptr_buf.is_null() {
        return;
    }
    unsafe {
        free(ptr_buf);
    }
}
