
use tokio;
use tokio::io::{AsyncBufReadExt, BufReader}; 
use tokio::sync::{Semaphore, SemaphorePermit};

use reqwest::{self, Client, redirect::Policy};

use std::error::Error;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn Error + 'static>>;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
#[command(about="Perform HTTP(S) HEAD requests en masse, logging status codes")]
struct Config {
    /// Maximum parallel connections to make
    #[arg(short, long, default_value="1000", value_name="CONNECTIONS")]
    connections: usize,

    /// Request timeout
    #[arg(short, long, default_value="10", value_name="SECONDS")]
    timeout: u64,

    /// Ignore TLS certificate validation
    #[arg(short='k', long="insecure")]
    no_verify: bool,
}

fn main() -> Result<()> { 

    let config = Config::parse();

    #[cfg(unix)]
    limit::check(config.connections as u64 + 10)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(config))

}

async fn run(conf: Config) -> Result<()> {

    let mut stdin = BufReader::new(tokio::io::stdin()).lines();
    let permits = conf.connections;

    let sem = Box::leak(Box::new(Semaphore::new(permits)));

    let client = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(conf.timeout))
        .danger_accept_invalid_certs(conf.no_verify)
        .build()?;

    while let Some(url) = stdin.next_line().await? {
        let permit = sem.acquire().await?;
        tokio::spawn(fetch(url, client.clone(), permit));
    } 

    let _ = sem.acquire_many(permits as u32).await?;

    Ok(())

}


async fn fetch(url: String, client: Client, permit: SemaphorePermit<'static>) -> () {
    match client.head(&url).send().await {
        Err(e) => {
            const TO_CLEAN: [char; 2] = ['\t', '\n'];
            let msg = e.to_string()
                .replace(&TO_CLEAN[..], " ");
            println!("{}\t000\t{}", url, msg)
        },
        Ok(r) => {
            let code = r.status();
            if code.is_redirection() {
                let dest = r.headers().get("Location")
                    .and_then(|d| d.to_str().ok())
                    .unwrap_or("?");
                println!("{}\t{}\t{}", url, code.as_str(), dest);
            } else {
                println!("{}\t{}", url, code);
            }
        }
    }
    drop(permit);
}

#[cfg(unix)]
mod limit {

    use super::Result;
    use rlimit::Resource;

    pub fn check(conns: u64) -> Result<()> {
        let (soft, hard) = Resource::NOFILE.get()?;

        if hard < conns {
            panic!("Error: not enough file descriptors (requested: {}, allowed: {})", conns, hard);
        }

        if soft < conns {
            eprintln!("Notice: Increasing fd soft limit from {} to {}.", soft, conns);
            Resource::NOFILE.set(conns, hard)?
        }

        Ok(())
    }
}
