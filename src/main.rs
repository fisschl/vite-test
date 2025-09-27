use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "s3-sync")]
#[command(about = "A simple CLI tool for syncing files with S3", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sync files with S3
    Sync,
    /// List files in S3
    List,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Sync => {
            println!("Syncing files...");
            // 这里可以添加实际的同步逻辑
        }
        Commands::List => {
            println!("Listing files...");
            // 这里可以添加实际的文件列表逻辑
        }
    }
}
