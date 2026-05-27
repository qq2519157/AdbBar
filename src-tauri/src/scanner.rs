use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;

#[derive(Serialize, Clone, Debug)]
pub struct ScanResult {
    pub ip: String,
    pub port: u16,
}

#[derive(Serialize, Clone)]
pub struct ScanProgress {
    pub scanned: u32,
    pub total: u32,
    pub found: Vec<ScanResult>,
}

pub async fn scan_subnet<F>(
    port: u16,
    emit_event: F,
) -> Result<Vec<ScanResult>, String>
where
    F: Fn(ScanProgress) + Send + Sync + 'static,
{
    let local_ip = local_ip_address::local_ip()
        .map_err(|e| format!("Failed to get local IP: {}", e))?;

    let subnet_base = match local_ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            Ipv4Addr::new(octets[0], octets[1], octets[2], 0)
        }
        IpAddr::V6(_) => return Err("IPv6 subnet scanning is not supported".to_string()),
    };

    let total: u32 = 254;
    let semaphore = Arc::new(Semaphore::new(128));
    let found = Arc::new(tokio::sync::Mutex::new(Vec::<ScanResult>::new()));
    let scanned = Arc::new(tokio::sync::Mutex::new(0u32));
    let emit = Arc::new(emit_event);

    let mut handles = Vec::new();

    for host in 1u8..=254 {
        let ip = Ipv4Addr::new(
            subnet_base.octets()[0],
            subnet_base.octets()[1],
            subnet_base.octets()[2],
            host,
        );
        let addr = format!("{}:{}", ip, port);
        let sem = semaphore.clone();
        let found = found.clone();
        let scanned = scanned.clone();
        let emit = emit.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let socket_addr: std::net::SocketAddr = addr.parse().unwrap();

            let result = tokio::time::timeout(
                Duration::from_millis(500),
                TcpStream::connect(&socket_addr),
            )
            .await;

            let mut found_list = found.lock().await;
            if let Ok(Ok(_stream)) = result {
                found_list.push(ScanResult {
                    ip: ip.to_string(),
                    port,
                });
            }

            let mut scanned_count = scanned.lock().await;
            *scanned_count += 1;
            let current_scanned = *scanned_count;

            emit(ScanProgress {
                scanned: current_scanned,
                total,
                found: found_list.clone(),
            });
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    let result = found.lock().await;
    Ok(result.clone())
}
