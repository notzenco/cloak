use std::fs;
use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use image::GenericImageView;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "cloak", about = "Steganography toolkit", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Embed encrypted data into a cover image
    Embed {
        /// Cover image path
        #[arg(short, long)]
        input: PathBuf,

        /// Data file to embed
        #[arg(short, long)]
        data: PathBuf,

        /// Output image path
        #[arg(short, long)]
        output: PathBuf,

        /// Passphrase (will prompt if not provided)
        #[arg(short, long)]
        passphrase: Option<String>,
    },

    /// Extract hidden data from a stego image
    Extract {
        /// Stego image path
        #[arg(short, long)]
        input: PathBuf,

        /// Output file path for extracted data
        #[arg(short, long)]
        output: PathBuf,

        /// Passphrase (will prompt if not provided)
        #[arg(short, long)]
        passphrase: Option<String>,
    },

    /// Analyze an image for steganographic content
    Analyze {
        /// Image path
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Show maximum embeddable payload size
    Capacity {
        /// Image path
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Launch interactive TUI analysis dashboard
    Inspect {
        /// Image path
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn get_passphrase(provided: Option<String>, confirm: bool) -> Result<String> {
    if let Some(p) = provided {
        return Ok(p);
    }

    let pass = rpassword::prompt_password("Passphrase: ").context("failed to read passphrase")?;

    if confirm {
        let pass2 = rpassword::prompt_password("Confirm passphrase: ")
            .context("failed to read passphrase confirmation")?;
        if pass != pass2 {
            bail!("passphrases do not match");
        }
    }

    if pass.is_empty() {
        bail!("passphrase cannot be empty");
    }

    Ok(pass)
}

fn make_progress(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

/// If the input format is lossy, correct the output path to use the lossless output extension.
fn resolve_output_path(input_format: cloak_core::ImageFormat, output: &std::path::Path) -> PathBuf {
    if input_format.is_lossy() {
        let output_format = input_format.output_format();
        let out_ext = output_format.extension();
        let out_str = output.to_string_lossy().to_lowercase();
        // If output has a lossy extension, correct it
        if out_str.ends_with(".jpg") || out_str.ends_with(".jpeg") || out_str.ends_with(".webp") {
            let stem = output.file_stem().unwrap_or_default();
            let parent = output.parent().unwrap_or(std::path::Path::new("."));
            let corrected = parent.join(format!("{}{}", stem.to_string_lossy(), out_ext));
            eprintln!(
                "Warning: lossy input format; output will be {} — saving as {}",
                out_ext.trim_start_matches('.').to_uppercase(),
                corrected.display()
            );
            return corrected;
        }
    }
    output.to_path_buf()
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Embed {
            input,
            data,
            output,
            passphrase,
        } => {
            let passphrase = get_passphrase(passphrase, true)?;

            let cover = fs::read(&input)
                .with_context(|| format!("failed to read cover image: {}", input.display()))?;
            let payload = fs::read(&data)
                .with_context(|| format!("failed to read data file: {}", data.display()))?;

            let path_str = input.to_string_lossy();
            let format = cloak_core::ImageFormat::detect(&cover, Some(&path_str))
                .context("format detection failed")?;

            let cap = cloak_core::capacity(&cover, Some(&path_str))
                .context("failed to calculate capacity")?;

            if payload.len() > cap {
                bail!(
                    "payload ({} bytes) exceeds capacity ({} bytes)",
                    payload.len(),
                    cap,
                );
            }

            let output = resolve_output_path(format, &output);

            if format.is_lossy() {
                eprintln!(
                    "Note: {} is a lossy format; stego output will be {}",
                    path_str,
                    format.output_format().extension().trim_start_matches('.').to_uppercase()
                );
            }

            let pb = make_progress("Embedding data...");
            let stego = cloak_core::embed(&cover, &payload, &passphrase, Some(&path_str))
                .context("embedding failed")?;
            pb.finish_with_message("Embedding complete");

            fs::write(&output, stego)
                .with_context(|| format!("failed to write output: {}", output.display()))?;

            println!("Embedded {} bytes into {}", payload.len(), output.display());
        }

        Command::Extract {
            input,
            output,
            passphrase,
        } => {
            let passphrase = get_passphrase(passphrase, false)?;

            let stego = fs::read(&input)
                .with_context(|| format!("failed to read stego image: {}", input.display()))?;

            let pb = make_progress("Extracting data...");
            let path_str = input.to_string_lossy();
            let data = cloak_core::extract(&stego, &passphrase, Some(&path_str))
                .context("extraction failed")?;
            pb.finish_with_message("Extraction complete");

            fs::write(&output, &data)
                .with_context(|| format!("failed to write output: {}", output.display()))?;

            println!("Extracted {} bytes to {}", data.len(), output.display());
        }

        Command::Analyze { input } => {
            let data = fs::read(&input)
                .with_context(|| format!("failed to read image: {}", input.display()))?;

            let path_str = input.to_string_lossy();
            let format = cloak_core::ImageFormat::detect(&data, Some(&path_str))
                .context("format detection failed")?;

            let cap = cloak_core::capacity(&data, Some(&path_str))
                .context("capacity calculation failed")?;

            let img = image::load_from_memory(&data).context("failed to decode image")?;
            let (w, h) = img.dimensions();

            println!("Image:      {}", input.display());
            println!("Format:     {format:?}");
            println!("Dimensions: {w} x {h}");
            println!("Pixels:     {}", w as u64 * h as u64);
            println!("Capacity:   {cap} bytes (usable for payload)");

            if let Ok(analysis) = cloak_core::analysis::analyze_image(&data) {
                println!();
                println!(
                    "Chi-square: {:.4} (p-value: {:.4})",
                    analysis.chi_square, analysis.p_value
                );
                if analysis.p_value < 0.05 {
                    println!("Result:     Likely contains hidden data (p < 0.05)");
                } else {
                    println!("Result:     No strong evidence of hidden data");
                }

                if let Some(rs) = &analysis.rs {
                    println!();
                    println!("RS Analysis:");
                    println!("  R_m:  {:.4}  S_m:  {:.4}", rs.r_m, rs.s_m);
                    println!("  R-m:  {:.4}  S-m:  {:.4}", rs.r_neg_m, rs.s_neg_m);
                    println!("  Estimated rate: {:.4}", rs.estimated_rate);
                }

                if let Some(sp) = &analysis.sample_pairs {
                    println!();
                    println!("Sample Pairs:");
                    println!("  Total pairs: {}  Close pairs: {}", sp.total_pairs, sp.close_pairs);
                    println!("  Estimated rate: {:.4}", sp.estimated_rate);
                }

                if let Some(ent) = &analysis.entropy {
                    println!();
                    println!("Shannon Entropy (bits):");
                    println!("  Red:   {:.4}  Green: {:.4}  Blue:  {:.4}", ent.red, ent.green, ent.blue);
                    println!("  Average: {:.4}", ent.average);
                }
            }
        }

        Command::Capacity { input } => {
            let data = fs::read(&input)
                .with_context(|| format!("failed to read image: {}", input.display()))?;

            let path_str = input.to_string_lossy();
            let cap = cloak_core::capacity(&data, Some(&path_str))
                .context("capacity calculation failed")?;

            println!("{cap} bytes");
        }

        Command::Inspect { input } => {
            let data = fs::read(&input)
                .with_context(|| format!("failed to read image: {}", input.display()))?;
            cloak_tui::run_tui(&data, &input.to_string_lossy())
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
    }

    Ok(())
}
