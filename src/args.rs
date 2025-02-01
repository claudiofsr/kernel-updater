// args.rs
use clap::Parser;

#[derive(Parser)]
pub struct Arguments {
    #[arg(short, long)]
    pub old: String,

    #[arg(short, long)]
    pub new: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    DkmsInstall,
    KernelCompile,
}
