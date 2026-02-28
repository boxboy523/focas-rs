use std::ffi::CString;
use std::sync::Mutex;

use thiserror::Error;

use crate::bindings::*;

// 1. bindgen으로 생성된 C 코드 모듈 가져오기
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[cfg(target_os = "linux")]
mod bindings {
    include!("bindings_linux.rs");
    // 또는 include!("bindings.rs"); (파일을 직접 생성했을 경우)
}

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[cfg(target_os = "windows")]
mod bindings {
    include!("bindings_windows.rs");
}

#[derive(Debug, Clone, Error)]
pub enum FocasError {
    #[error("Focas API error: {0}")]
    ApiError(i16),
    #[error("Invalid IP address format")]
    InvalidIpFormat,
    #[error("Connection failed with error code: {0}")]
    ConnectionFailed(i16),
}

pub struct FocasClient {
    handle: u16,
    ip: String,
    port: u16,
}

impl Drop for FocasClient {
    fn drop(&mut self) {
        unsafe {
            cnc_freelibhndl(self.handle);
        }
    }
}

impl FocasClient {
    pub fn new(ip: &str, port: u16) -> Result<Self, FocasError> {
        let mut hndl: u16 = 0;
        let ret = unsafe {
            let ip_cstr = CString::new(ip).unwrap();
            cnc_allclibhndl3(ip_cstr.as_ptr(), port, 5, &mut hndl)
        };

        if ret != 0 {
            return Err(FocasError::ConnectionFailed(ret));
        }

        Ok(FocasClient {
            handle: hndl,
            ip: ip.to_string(),
            port,
        })
    }

    pub fn sysinfo(&self) -> Result<ODBSYS, FocasError> {
        let mut info: ODBSYS = ODBSYS::default();

        let ret = unsafe { cnc_sysinfo(self.handle, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(info)
    }

    pub fn rdtofs(&self, ofs_number: i16, ofs_type: i16) -> Result<ODBTOFS, FocasError> {
        let mut info: ODBTOFS = ODBTOFS::default();

        let ret = unsafe { cnc_rdtofs(self.handle, ofs_number, ofs_type, 8, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(info)
    }

    pub fn wrtofs(&self, ofs_number: i16, ofs_type: i16, offset: i64) -> Result<(), FocasError> {
        let ret = unsafe { cnc_wrtofs(self.handle, ofs_number, ofs_type, 8, offset) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
