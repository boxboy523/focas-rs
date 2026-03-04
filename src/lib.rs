use std::ffi::CString;

use crate::bindings::*;
use thiserror::Error;

// 1. bindgen으로 생성된 C 코드 모듈 가져오기
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[cfg(target_os = "linux")]
mod bindings {
    include!("bindings_linux.rs");
}

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[cfg(target_os = "windows")]
mod bindings {
    include!("bindings_windows.rs");
}

#[cfg(target_os = "linux")]
pub fn cnc_startup(log_path: &str) -> i16 {
    use std::path::Path;

    let path = Path::new(log_path);
    if !path.exists() {
        use std::fs::File;

        match File::create(path) {
            Ok(_) => println!("Log file created at: {}", log_path),
            Err(e) => {
                eprintln!("Failed to create log file: {}", e);
                return -1; // 파일 생성 실패 시 -1 반환
            }
        }
    }
    let log_cstr = CString::new(log_path).unwrap();
    unsafe { cnc_startupprocess(3, log_cstr.as_ptr()) }
}

#[cfg(target_os = "linux")]
pub fn cnc_exit() -> i16 {
    unsafe { cnc_exitprocess() }
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

impl FocasError {
    pub fn is_fatal(&self) -> bool {
        match self {
            FocasError::ApiError(code) => *code < 0 && *code != -1, // -1 is busy
            FocasError::InvalidIpFormat => true,
            FocasError::ConnectionFailed(_) => true,
        }
    }
}

pub struct FocasClient {
    ip: String,
    port: u16,
}

impl FocasClient {
    pub fn new(ip: &str, port: u16) -> Result<Self, FocasError> {
        Ok(FocasClient {
            ip: ip.to_string(),
            port,
        })
    }

    fn get_handle(&self) -> Result<u16, FocasError> {
        let mut handle: u16 = 0;
        let ip_cstr = CString::new(self.ip.clone()).map_err(|_| FocasError::InvalidIpFormat)?;
        let ret = unsafe { cnc_allclibhndl3(ip_cstr.as_ptr(), self.port, 10, &mut handle) };
        if ret != 0 {
            return Err(FocasError::ConnectionFailed(ret));
        }
        Ok(handle)
    }

    fn free_handle(&self, handle: u16) -> Result<(), FocasError> {
        let ret = unsafe { cnc_freelibhndl(handle) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }
        Ok(())
    }

    pub fn sysinfo(&self) -> Result<ODBSYS, FocasError> {
        let mut info: ODBSYS = ODBSYS::default();

        let handle = self.get_handle()?;
        let ret = unsafe { cnc_sysinfo(handle.try_into().unwrap(), &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }
        let _ = self.free_handle(handle);
        Ok(info)
    }

    pub fn rdtofs(&self, ofs_number: i16, ofs_type: i16) -> Result<ODBTOFS, FocasError> {
        let mut info: ODBTOFS = ODBTOFS::default();

        let handle = self.get_handle()?;
        let ret = unsafe { cnc_rdtofs(handle, ofs_number, ofs_type, 8, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }
        let _ = self.free_handle(handle);
        Ok(info)
    }

    pub fn wrtofs(&self, ofs_number: i16, ofs_type: i16, offset: i32) -> Result<(), FocasError> {
        let handle = self.get_handle()?;
        let ret = unsafe { cnc_wrtofs(handle, ofs_number, ofs_type, 8, offset) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }
        let _ = self.free_handle(handle);
        Ok(())
    }

    pub fn rdlife(&self, life_number: i16) -> Result<ODBTLIFE3, FocasError> {
        let mut info = ODBTLIFE3::default();
        let handle = self.get_handle()?;
        let ret = unsafe { cnc_rdlife(handle, life_number, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }
        let _ = self.free_handle(handle);
        Ok(info)
    }

    pub fn rdcount(&self, count_number: i16) -> Result<ODBTLIFE3, FocasError> {
        let mut info = ODBTLIFE3::default();
        let handle = self.get_handle()?;
        let ret = unsafe { cnc_rdcount(handle, count_number, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }
        let _ = self.free_handle(handle);
        Ok(info)
    }
}
