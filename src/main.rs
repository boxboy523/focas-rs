use focas_rs::FocasShell;

#[tokio::main]
async fn main() {
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
    let client = FocasShell::new("192.168.0.1", 8193).expect("Failed to connect to CNC");
    let tofs = client.rdtofs(1, 2).await.unwrap();
    println!("Current tool offset: {:?}", tofs);
    client.wrtofs(1, 2, 10).await.unwrap();
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
