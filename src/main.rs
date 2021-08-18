
use tokio;
use tokio::io::{AsyncBufReadExt, BufReader}; 
use tokio::sync::{Semaphore, SemaphorePermit};

use reqwest::{self, Client, redirect::Policy};

use std::error::Error;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn Error + 'static>>;

use clap::{Clap, AppSettings};

#[derive(Clap)]
#[clap(version="1.0", author="Max <max@xolus.net>")]
#[clap(about="Perform https HEAD requests en masse, logging status codes")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Config {
    #[clap(short, default_value="1024", value_name="CONNECTIONS", about="Maximum parallel connections to make")]
    connections: usize,
    #[clap(short, default_value="10", value_name="SECONDS", about="Request timeout")]
    timeout: u64,
    #[clap(short, about="Ignore HTTPS certificate validation")]
    no_verify: bool,
}

fn main() -> Result<()> {

    let config = Config::parse();
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
        Err(e) => println!("{}\t000 {}", url, e),
        Ok(r) => {
            let code = r.status();
            if code.is_redirection() {
                println!("{}\t{} {}", url, code.as_str(), r.url());
            } else {
                println!("{}\t{}", url, code);
            }
        }
    }
    drop(permit);
}