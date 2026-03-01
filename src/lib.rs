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

    pub fn get_handle(&self) -> u16 {
        self.handle
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

    pub fn wrtofs(&self, ofs_number: i16, ofs_type: i16, offset: i32) -> Result<(), FocasError> {
        let ret = unsafe { cnc_wrtofs(self.handle, ofs_number, ofs_type, 8, offset) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(())
    }

    pub fn rdlife(&self, life_number: i16) -> Result<ODBTLIFE3, FocasError> {
        let mut info = ODBTLIFE3::default();

        let ret = unsafe { cnc_rdlife(self.handle, life_number, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(info)
    }

    pub fn rdcount(&self, count_number: i16) -> Result<ODBTLIFE3, FocasError> {
        let mut info = ODBTLIFE3::default();

        let ret = unsafe { cnc_rdcount(self.handle, count_number, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
