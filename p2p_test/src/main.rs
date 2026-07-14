use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::io::{Read, Write};
use std::net::TcpStream;
use kona_net::driver::NetworkDriver;
use tokio::sync::mpsc;
use discv5::ListenConfig;
use libp2p::{Multiaddr, multiaddr::Protocol};

fn get_public_ip() -> Option<Ipv4Addr> {
    let mut stream = TcpStream::connect("api.ipify.org:80").ok()?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok()?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(5))).ok()?;
    let request = b"GET / HTTP/1.0\r\nHost: api.ipify.org\r\nConnection: close\r\n\r\n";
    stream.write_all(request).ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    let ip = response.lines().last()?.trim().to_string();
    ip.parse().ok()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .init();

    println!("=== P2P Test: Joining Base P2P network (kona-net) ===");

    let signer = alloy_primitives::address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9222);

    println!("Chain:  Base mainnet (ID 8453)");
    println!("Signer: {signer}");
    println!("Socket: {socket}");

    // Detect our public IP so we can advertise it on the libp2p swarm
    let public_ip = get_public_ip();
    match public_ip {
        Some(ip) => println!("Detected public IP: {ip}"),
        None => println!("Could not detect public IP (will rely on discv5 ENR auto-update)"),
    }

    // Custom discv5 config: faster ENR IP update, no expiry from missing incoming connections
    let listen_config = ListenConfig::from_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9222);
    let discv5_config = discv5::ConfigBuilder::new(listen_config)
        .enr_peer_update_min(2)
        .auto_nat_listen_duration(None)
        .build();

    let mut driver = match NetworkDriver::builder()
        .with_chain_id(8453u64)
        .with_unsafe_block_signer(signer)
        .with_gossip_addr(socket)
        .with_discovery_config(discv5_config)
        .build()
    {
        Ok(d) => {
            println!("NetworkDriver built successfully!");
            d
        }
        Err(e) => {
            eprintln!("Failed to build NetworkDriver: {e:?}");
            return;
        }
    };

    // Add our public IP as an external address on the libp2p swarm.
    // This ensures the identify protocol advertises the correct source IP,
    // preventing remote peers from dropping our connection due to IP mismatch.
    if let Some(ip) = public_ip {
        let mut addr = Multiaddr::empty();
        addr.push(Protocol::Ip4(ip));
        addr.push(Protocol::Tcp(9222));
        let _ = driver.gossip.swarm.add_external_address(addr);
        println!("Added external address to swarm: {ip}:9222");
    }

    let block_recv = driver.take_unsafe_block_recv();

    println!("Starting NetworkDriver...");
    match driver.start() {
        Ok(()) => println!("NetworkDriver started! Listening for gossip events..."),
        Err(e) => {
            eprintln!("Failed to start NetworkDriver: {e:?}");
            return;
        }
    }

    match block_recv {
        Some(rx) => {
            println!("\n=== Block receiver active! Waiting for blocks... ===\n");
            let (tx, mut async_rx) = mpsc::unbounded_channel();
            tokio::task::spawn_blocking(move || {
                loop {
                    match rx.recv() {
                        Ok(block) => {
                            if tx.send(block).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
            let mut count = 0u64;
            loop {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(120),
                    async_rx.recv(),
                )
                .await
                {
                    Ok(Some(block)) => {
                        count += 1;
                        println!("[#{count}] Received block: {block:?}");
                        if count >= 5 {
                            println!("\n=== Got {count} blocks — kona-net WORKS! ===");
                            break;
                        }
                    }
                    Ok(None) => {
                        println!("Block channel closed.");
                        break;
                    }
                    Err(_) => {
                        println!("[timeout] No block received in 120s");
                    }
                }
            }
        }
        None => {
            eprintln!("No block receiver available (already taken or not initialized)");
        }
    }

    println!("\nTest complete. Keeping process alive for inspection...");
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
