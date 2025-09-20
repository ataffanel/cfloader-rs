use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use cfloader::{Bllink, CFLoader};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::fs;

#[derive(Parser)]
#[command(name = "cfload")]
#[command(about = "A CLI tool for Crazyflie 2.x bootloader operations")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get info of the full platform and print it to the user
    Info,
    /// Flash a binary file to a specific platform
    Flash {
        /// Binary file to flash
        #[arg(short, long)]
        file: PathBuf,
        /// Platform to flash (stm32 or nrf51)
        #[arg(short, long)]
        platform: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize Bllink (will open Crazyradio internally)
    let bllink = Bllink::new(None).await?;

    match &cli.command {
        Commands::Info => {
            println!("Connecting to Crazyflie 2.x bootloaders...");
            
            // Initialize CFLoader which will connect to both bootloaders
            let cfloader = CFLoader::new(bllink).await?;
            
            println!("Platform Information:");
            println!("====================");
            
            // Get and display STM32 info
            let stm32_info = cfloader.stm32_info();
            println!("STM32F405 Bootloader:");
            println!("  Page size: {} bytes", stm32_info.page_size());
            println!("  Buffer pages: {}", stm32_info.n_buff_page());
            println!("  Flash pages: {}", stm32_info.n_flash_page());
            println!("  Flash start: {}", stm32_info.flash_start());
            println!("  Protocol version: {}", stm32_info.version());
            
            // Get and display nRF51 info
            let nrf51_info = cfloader.nrf51_info();
            println!("\nnRF51822 Bootloader:");
            println!("  Page size: {} bytes", nrf51_info.page_size());
            println!("  Buffer pages: {}", nrf51_info.n_buff_page());
            println!("  Flash pages: {}", nrf51_info.n_flash_page());
            println!("  Flash start: {}", nrf51_info.flash_start());
            println!("  Protocol version: {}", nrf51_info.version());
        }
        Commands::Flash { file, platform } => {
            println!("Flashing {} to {} platform...", file.display(), platform);
            
            // Read the binary file
            let firmware_data = fs::read(file).await?;
            println!("Read {} bytes from {}", firmware_data.len(), file.display());
            
            // Initialize CFLoader
            let mut cfloader = CFLoader::new(bllink).await?;
            
            // Create progress bar
            let progress_bar = ProgressBar::new(firmware_data.len() as u64);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );

            // Flash based on platform using correct start addresses
            match platform.to_lowercase().as_str() {
                "stm32" => {
                    let stm32_info = cfloader.stm32_info();
                    let start_address = stm32_info.flash_start() as u32 * stm32_info.page_size() as u32;
                    println!("Flashing STM32F405 starting at address 0x{:08X}...", start_address);
                    
                    // Create progress callback
                    let pb = progress_bar.clone();
                    let progress_callback = move |bytes_written: usize, _total_bytes: usize| {
                        pb.set_position(bytes_written as u64);
                    };
                    
                    cfloader.flash_stm32_with_progress(start_address, &firmware_data, Some(progress_callback)).await?;
                    progress_bar.finish_with_message("STM32F405 flashed successfully!");
                }
                "nrf51" => {
                    let nrf51_info = cfloader.nrf51_info();
                    let start_address = nrf51_info.flash_start() as u32 * nrf51_info.page_size() as u32;
                    println!("Flashing nRF51822 starting at address 0x{:08X}...", start_address);
                    
                    // Create progress callback
                    let pb = progress_bar.clone();
                    let progress_callback = move |bytes_written: usize, _total_bytes: usize| {
                        pb.set_position(bytes_written as u64);
                    };
                    
                    cfloader.flash_nrf51_with_progress(start_address, &firmware_data, Some(progress_callback)).await?;
                    progress_bar.finish_with_message("nRF51822 flashed successfully!");
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid platform '{}'. Use 'stm32' or 'nrf51'", 
                        platform
                    ));
                }
            }
        }
    }

    Ok(())
}