#![forbid(unsafe_code)]

use clap::{command, Arg, Parser};

mod error;
mod protocol;
mod proxy;

#[derive(Parser, Debug)] 
#[command(term_width = 0)] // Just to make testing across clap features easier
struct Args {
    
    /// Allow invalid UTF-8 paths
    #[arg(short = 'c', value_name = "config", value_hint = clap::ValueHint::FilePath)]
    config: std::path::PathBuf,

}



#[tokio::main]
async fn main() {

    env_logger::init();

    let args = Args::parse();

    let filename = args.config.to_string();
    if let Err(e) = proxy::launch_from_config_filename(filename).await {
        println!("failed to launch proxy: {}", e);
    }
}
