use std::ffi::CString;

use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::sleep;

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

impl FocasError {
    pub fn is_fatal(&self) -> bool {
        match self {
            FocasError::ApiError(code) => *code < 0 && *code != -1, // -1 is busy
            FocasError::InvalidIpFormat => true,
            FocasError::ConnectionFailed(_) => true,
        }
    }
}

#[derive(Debug)]
pub enum FocasRequest {
    SYSINFO {
        responder: oneshot::Sender<Result<ODBSYS, FocasError>>,
    },
    RDTOFS {
        ofs_number: i16,
        ofs_type: i16,
        responder: oneshot::Sender<Result<ODBTOFS, FocasError>>,
    },
    WRTOFS {
        ofs_number: i16,
        ofs_type: i16,
        offset: i32,
        responder: oneshot::Sender<Result<(), FocasError>>,
    },
    RDLIFE {
        life_number: i16,
        responder: oneshot::Sender<Result<ODBTLIFE3, FocasError>>,
    },
    RDCOUNT {
        count_number: i16,
        responder: oneshot::Sender<Result<ODBTLIFE3, FocasError>>,
    },
    GETIP {
        responder: oneshot::Sender<Result<(String, u16), FocasError>>,
    },
    STOP {
        responder: oneshot::Sender<Result<(), FocasError>>,
    },
}

struct FocasClient {
    handle: u16,
    ip: String,
    port: u16,
    running: bool,
    receiver: mpsc::Receiver<FocasRequest>,
}

impl Drop for FocasClient {
    fn drop(&mut self) {
        let ret = unsafe { cnc_freelibhndl(self.handle) };
        if ret != 0 {
            eprintln!("Failed to free Focas handle: {}", ret);
        }
    }
}

impl FocasClient {
    fn new(
        ip: &str,
        port: u16,
        receiver: mpsc::Receiver<FocasRequest>,
    ) -> Result<Self, FocasError> {
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
            running: true,
            receiver,
        })
    }

    fn reconnect(&mut self) -> Result<(), FocasError> {
        if self.handle != 0 {
            let ret = unsafe { cnc_freelibhndl(self.handle) };
            if ret != 0 {
                eprintln!("Failed to free old Focas handle: {}", ret);
            }
            self.handle = 0;
        }
        let mut hndl: u16 = 0;
        let ret = unsafe {
            let ip_cstr = CString::new(self.ip.clone()).unwrap();
            cnc_allclibhndl3(ip_cstr.as_ptr(), self.port, 5, &mut hndl)
        };

        if ret != 0 {
            return Err(FocasError::ConnectionFailed(ret));
        }

        self.handle = hndl;
        Ok(())
    }

    async fn run(&mut self) {
        while let Some(request) = self.receiver.recv().await {
            if let Err(e) = self.match_request(request) {
                if e.is_fatal() {
                    eprintln!("Fatal error encountered. Attempting to reconnect...");
                    sleep(std::time::Duration::from_secs(5)).await;
                    if let Err(reconnect_err) = self.reconnect() {
                        eprintln!("Reconnection failed: {}", reconnect_err);
                        break;
                    } else {
                        eprintln!("Reconnected successfully.");
                    }
                }
            }
            if !self.running {
                break;
            }
        }
    }

    fn match_request(&mut self, request: FocasRequest) -> Result<(), FocasError> {
        let mut err = None;
        match request {
            FocasRequest::SYSINFO { responder } => {
                let result = self.sysinfo();
                if let Err(e) = &result {
                    err = Some(Err(e.clone().into()));
                }
                let _ = responder.send(result);
            }
            FocasRequest::RDTOFS {
                ofs_number,
                ofs_type,
                responder,
            } => {
                let result = self.rdtofs(ofs_number, ofs_type);
                if let Err(e) = &result {
                    err = Some(Err(e.clone().into()));
                }
                let _ = responder.send(result);
            }
            FocasRequest::WRTOFS {
                ofs_number,
                ofs_type,
                offset,
                responder,
            } => {
                let result = self.wrtofs(ofs_number, ofs_type, offset);
                if let Err(e) = &result {
                    err = Some(Err(e.clone().into()));
                }
                let _ = responder.send(result);
            }
            FocasRequest::RDLIFE {
                life_number,
                responder,
            } => {
                let result = self.rdlife(life_number);
                let _ = responder.send(result);
                if let Err(e) = &result {
                    err = Some(Err(e.clone().into()));
                }
            }
            FocasRequest::RDCOUNT {
                count_number,
                responder,
            } => {
                let result = self.rdcount(count_number);
                if let Err(e) = &result {
                    err = Some(Err(e.clone().into()));
                }
                let _ = responder.send(result);
            }
            FocasRequest::GETIP { responder } => {
                let result = Ok((self.ip.clone(), self.port));
                let _ = responder.send(result);
            }
            FocasRequest::STOP { responder } => {
                let result = Ok(());
                self.running = false;
                let _ = responder.send(result);
            }
        }
        if let Some(e) = err {
            e
        } else {
            Ok(())
        }
    }

    fn get_handle(&self) -> u16 {
        self.handle
    }

    fn sysinfo(&self) -> Result<ODBSYS, FocasError> {
        let mut info: ODBSYS = ODBSYS::default();

        let ret = unsafe { cnc_sysinfo(self.handle, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(info)
    }

    fn rdtofs(&self, ofs_number: i16, ofs_type: i16) -> Result<ODBTOFS, FocasError> {
        let mut info: ODBTOFS = ODBTOFS::default();

        let ret = unsafe { cnc_rdtofs(self.handle, ofs_number, ofs_type, 8, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(info)
    }

    fn wrtofs(&self, ofs_number: i16, ofs_type: i16, offset: i32) -> Result<(), FocasError> {
        let ret = unsafe { cnc_wrtofs(self.handle, ofs_number, ofs_type, 8, offset) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(())
    }

    fn rdlife(&self, life_number: i16) -> Result<ODBTLIFE3, FocasError> {
        let mut info = ODBTLIFE3::default();

        let ret = unsafe { cnc_rdlife(self.handle, life_number, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(info)
    }

    fn rdcount(&self, count_number: i16) -> Result<ODBTLIFE3, FocasError> {
        let mut info = ODBTLIFE3::default();

        let ret = unsafe { cnc_rdcount(self.handle, count_number, &mut info) };
        if ret != 0 {
            return Err(FocasError::ApiError(ret));
        }

        Ok(info)
    }
}

pub struct FocasShell {
    thread: JoinHandle<()>,
    sender: mpsc::Sender<FocasRequest>,
}

impl FocasShell {
    pub fn new(ip: &str, port: u16) -> Result<Self, FocasError> {
        let (sender, receiver) = mpsc::channel(32);
        let mut client = FocasClient::new(ip, port, receiver)?;
        let thread = tokio::spawn(async move { client.run().await });
        Ok(FocasShell { thread, sender })
    }

    pub async fn sysinfo(&self) -> anyhow::Result<ODBSYS> {
        if self.thread.is_finished() {
            return Err(anyhow::anyhow!("FocasShell thread has terminated"));
        }
        let (responder, response) = oneshot::channel();
        let request = FocasRequest::SYSINFO { responder };
        self.sender.send(request).await?;
        response
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Focas API error: {}", e))
    }

    pub async fn rdtofs(&self, ofs_number: i16, ofs_type: i16) -> anyhow::Result<ODBTOFS> {
        if self.thread.is_finished() {
            return Err(anyhow::anyhow!("FocasShell thread has terminated"));
        }
        let (responder, response) = oneshot::channel();
        let request = FocasRequest::RDTOFS {
            ofs_number,
            ofs_type,
            responder,
        };
        self.sender.send(request).await?;
        response
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Focas API error: {}", e))
    }

    pub async fn wrtofs(&self, ofs_number: i16, ofs_type: i16, offset: i32) -> anyhow::Result<()> {
        if self.thread.is_finished() {
            return Err(anyhow::anyhow!("FocasShell thread has terminated"));
        }
        let (responder, response) = oneshot::channel();
        let request = FocasRequest::WRTOFS {
            ofs_number,
            ofs_type,
            offset,
            responder,
        };
        self.sender.send(request).await?;
        response
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Focas API error: {}", e))
    }

    pub async fn rdlife(&self, life_number: i16) -> anyhow::Result<ODBTLIFE3> {
        if self.thread.is_finished() {
            return Err(anyhow::anyhow!("FocasShell thread has terminated"));
        }
        let (responder, response) = oneshot::channel();
        let request = FocasRequest::RDLIFE {
            life_number,
            responder,
        };
        self.sender.send(request).await?;
        response
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Focas API error: {}", e))
    }

    pub async fn rdcount(&self, count_number: i16) -> anyhow::Result<ODBTLIFE3> {
        if self.thread.is_finished() {
            return Err(anyhow::anyhow!("FocasShell thread has terminated"));
        }
        let (responder, response) = oneshot::channel();
        let request = FocasRequest::RDCOUNT {
            count_number,
            responder,
        };
        self.sender.send(request).await?;
        response
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Focas API error: {}", e))
    }

    pub async fn get_ip(&self) -> anyhow::Result<(String, u16)> {
        if self.thread.is_finished() {
            return Err(anyhow::anyhow!("FocasShell thread has terminated"));
        }
        let (responder, response) = oneshot::channel();
        let request = FocasRequest::GETIP { responder };
        self.sender.send(request).await?;
        response
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Focas API error: {}", e))
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        if self.thread.is_finished() {
            return Err(anyhow::anyhow!("FocasShell thread has already terminated"));
        }
        let (responder, response) = oneshot::channel();
        let request = FocasRequest::STOP { responder };
        self.sender.send(request).await?;
        response
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive response: {}", e))?
            .map_err(|e| anyhow::anyhow!("Focas API error: {}", e))
    }

    pub fn is_running(&self) -> bool {
        !self.thread.is_finished()
    }
}

impl Drop for FocasShell {
    fn drop(&mut self) {
        if self.is_running() {
            self.thread.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
