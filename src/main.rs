mod error;
mod kube;
mod probe;

// how many ups will be re-added to ep
const HEALTHY_UPS: u8 = 1;
// how many downs will be removed from ep
const REMOVED_DOWNS: u8 = 3;
// ms to consider an ep is down
const CONNECT_TIMEOUT: u64 = 100;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();
    println!("Hello, world!");
    Ok(())
}

fn init() {
    env_logger::init();
}
