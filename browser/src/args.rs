use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct BrowserArgs {
    #[arg(short, long, default_value_t = false)]
    pub debug_inspector: bool,

    #[arg(short, long)]
    pub view: Option<PathBuf>,
}