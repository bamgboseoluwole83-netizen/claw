use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use kona_net::driver::NetworkDriver;

#[tokio::main]
async fn main() {
    println!("=== P2P Test: Joining Base P2P network (kona-net) ===");

    let signer = alloy_primitives::address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9222);

    println!("Chain:  Base mainnet (ID 8453)");
    println!("Signer: {signer}");
    println!("Socket: {socket}");

    let mut driver = match NetworkDriver::builder()
        .with_chain_id(8453u64)
        .with_unsafe_block_signer(signer)
        .with_gossip_addr(socket)
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

    // Take the unsafe block receiver channel before starting
    let block_recv = driver.take_unsafe_block_recv();

    println!("Starting NetworkDriver...");
    match driver.start().await {
        Ok(()) => println!("NetworkDriver started! Listening for gossip events..."),
        Err(e) => {
            eprintln!("Failed to start NetworkDriver: {e:?}");
            return;
        }
    }

    // Receive blocks from the gossipsub channel
    match block_recv {
        Some(mut rx) => {
            println!("\n=== Block receiver active! Waiting for blocks... ===\n");
            let mut count = 0u64;
            loop {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(120),
                    rx.recv(),
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
                        println!("[timeout] No block received in 120s (this is normal — Base produces blocks every 2s)");
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
