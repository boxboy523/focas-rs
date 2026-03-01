use focas_rs::FocasClient;

fn main() {
    #[cfg(target_os = "linux")]
    {
        let log_path = "./focas2.log";
        let startup_result = focas_rs::cnc_startup(log_path);
        if startup_result != 0 {
            eprintln!(
                "Failed to start CNC process, error code: {}",
                startup_result
            );
            return;
        }
        println!("CNC process started successfully");
    }
    let client = FocasClient::new("192.168.0.10", 8193).unwrap();
    println!("Connected to CNC, handle: {}", client.get_handle());
    let tofs = client.rdtofs(1, 2).unwrap();
    println!("Current tool offset: {:?}", tofs);
    client.wrtofs(1, 2, 10).unwrap();
    println!("TOFS: {:?}", tofs);
    #[cfg(target_os = "linux")]
    {
        let exit_result = focas_rs::cnc_exit();
        if exit_result != 0 {
            eprintln!("Failed to exit CNC process, error code: {}", exit_result);
        } else {
            println!("CNC process exited successfully");
        }
    }
}
