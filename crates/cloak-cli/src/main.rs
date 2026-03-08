use std::fs;
use std::io::{self, Read as _, Write as _};
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
        /// Cover image path (use "-" for stdin)
        #[arg(short, long)]
        input: String,

        /// Data file to embed (use "-" for stdin)
        #[arg(short, long)]
        data: String,

        /// Output image path (use "-" for stdout)
        #[arg(short, long)]
        output: String,

        /// Passphrase (will prompt if not provided)
        #[arg(short, long)]
        passphrase: Option<String>,

        /// Bits per channel (1-4)
        #[arg(long, default_value = "1", value_parser = clap::value_parser!(u8).range(1..=4))]
        bit_depth: u8,

        /// Randomize pixel traversal order
        #[arg(long)]
        randomize: bool,
    },

    /// Extract hidden data from a stego image
    Extract {
        /// Stego image path (use "-" for stdin)
        #[arg(short, long)]
        input: String,

        /// Output file path for extracted data (use "-" for stdout)
        #[arg(short, long)]
        output: String,

        /// Passphrase (will prompt if not provided)
        #[arg(short, long)]
        passphrase: Option<String>,

        /// Bits per channel (must match embed setting)
        #[arg(long, default_value = "1", value_parser = clap::value_parser!(u8).range(1..=4))]
        bit_depth: u8,

        /// Must match the --randomize flag used during embedding
        #[arg(long)]
        randomize: bool,
    },

    /// Analyze an image for steganographic content
    Analyze {
        /// Image path or glob pattern (e.g., "photos/*.png")
        #[arg(short, long)]
        input: String,
    },

    /// Show maximum embeddable payload size
    Capacity {
        /// Image path or glob pattern (e.g., "covers/*.png")
        #[arg(short, long)]
        input: String,

        /// Bits per channel (1-4)
        #[arg(long, default_value = "1", value_parser = clap::value_parser!(u8).range(1..=4))]
        bit_depth: u8,
    },

    /// Launch interactive TUI analysis dashboard
    Inspect {
        /// Image path
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Batch embed data into multiple cover images
    BatchEmbed {
        /// Directory containing cover images
        #[arg(long)]
        input_dir: PathBuf,

        /// Directory containing data files (matched by stem name)
        #[arg(long)]
        data_dir: PathBuf,

        /// Output directory for stego images
        #[arg(long)]
        output_dir: PathBuf,

        /// Passphrase (will prompt if not provided)
        #[arg(short, long)]
        passphrase: Option<String>,

        /// Bits per channel (1-4)
        #[arg(long, default_value = "1", value_parser = clap::value_parser!(u8).range(1..=4))]
        bit_depth: u8,

        /// Randomize pixel traversal order
        #[arg(long)]
        randomize: bool,
    },

    /// Batch extract data from multiple stego images
    BatchExtract {
        /// Directory containing stego images
        #[arg(long)]
        input_dir: PathBuf,

        /// Output directory for extracted data
        #[arg(long)]
        output_dir: PathBuf,

        /// Passphrase (will prompt if not provided)
        #[arg(short, long)]
        passphrase: Option<String>,

        /// Bits per channel (must match embed setting)
        #[arg(long, default_value = "1", value_parser = clap::value_parser!(u8).range(1..=4))]
        bit_depth: u8,

        /// Must match the --randomize flag used during embedding
        #[arg(long)]
        randomize: bool,
    },
}

/// Read from a file or stdin if path is "-".
fn read_input(path: &str) -> Result<Vec<u8>> {
    if path == "-" {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .context("failed to read from stdin")?;
        Ok(buf)
    } else {
        fs::read(path).with_context(|| format!("failed to read: {path}"))
    }
}

/// Write to a file or stdout if path is "-".
fn write_output(path: &str, data: &[u8]) -> Result<()> {
    if path == "-" {
        io::stdout()
            .write_all(data)
            .context("failed to write to stdout")?;
        io::stdout().flush().context("failed to flush stdout")?;
        Ok(())
    } else {
        fs::write(path, data).with_context(|| format!("failed to write: {path}"))
    }
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

fn resolve_output_path(
    input_format: cloak_core::ImageFormat,
    output: &str,
) -> String {
    if output == "-" {
        return output.to_string();
    }
    if input_format.is_lossy() {
        let output_format = input_format.output_format();
        let out_ext = output_format.extension();
        let out_lower = output.to_lowercase();
        if out_lower.ends_with(".jpg") || out_lower.ends_with(".jpeg") || out_lower.ends_with(".webp") {
            let path = std::path::Path::new(output);
            let stem = path.file_stem().unwrap_or_default();
            let parent = path.parent().unwrap_or(std::path::Path::new("."));
            let corrected = parent.join(format!("{}{}", stem.to_string_lossy(), out_ext));
            let corrected_str = corrected.to_string_lossy().to_string();
            eprintln!(
                "Warning: lossy input format; output will be {} — saving as {}",
                out_ext.trim_start_matches('.').to_uppercase(),
                corrected_str
            );
            return corrected_str;
        }
    }
    output.to_string()
}

/// Resolve input string as either a literal path or a glob pattern.
fn resolve_inputs(input: &str) -> Result<Vec<PathBuf>> {
    let paths: Vec<PathBuf> = glob::glob(input)
        .context("invalid glob pattern")?
        .filter_map(|r| r.ok())
        .filter(|p| p.is_file())
        .collect();

    if paths.is_empty() {
        let p = PathBuf::from(input);
        if p.exists() {
            return Ok(vec![p]);
        }
        bail!("no files matched: {input}");
    }

    Ok(paths)
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
            bit_depth,
            randomize,
        } => {
            if input == "-" && data == "-" {
                bail!("cannot read both cover image and data from stdin");
            }

            let passphrase = get_passphrase(passphrase, true)?;
            let options = cloak_core::EmbedOptions {
                bit_depth,
                randomized: randomize,
            };

            let cover = read_input(&input)?;
            let payload = read_input(&data)?;

            let path_hint = if input == "-" { None } else { Some(input.as_str()) };
            let format = cloak_core::ImageFormat::detect(&cover, path_hint)
                .context("format detection failed")?;

            let cap = cloak_core::capacity(&cover, path_hint, &options)
                .context("failed to calculate capacity")?;

            if payload.len() > cap {
                bail!(
                    "payload ({} bytes) exceeds capacity ({} bytes)",
                    payload.len(),
                    cap,
                );
            }

            let output = resolve_output_path(format, &output);

            if format.is_lossy() && output != "-" {
                eprintln!(
                    "Note: lossy input format; stego output will be {}",
                    format
                        .output_format()
                        .extension()
                        .trim_start_matches('.')
                        .to_uppercase()
                );
            }

            // Only show progress spinner when not piping to stdout
            let pb = if output != "-" {
                Some(make_progress("Embedding data..."))
            } else {
                None
            };

            let stego =
                cloak_core::embed(&cover, &payload, &passphrase, path_hint, &options)
                    .context("embedding failed")?;

            if let Some(pb) = pb {
                pb.finish_with_message("Embedding complete");
            }

            write_output(&output, &stego)?;

            if output != "-" {
                println!("Embedded {} bytes into {}", payload.len(), output);
            }
        }

        Command::Extract {
            input,
            output,
            passphrase,
            bit_depth,
            randomize,
        } => {
            let passphrase = get_passphrase(passphrase, false)?;
            let options = cloak_core::EmbedOptions {
                bit_depth,
                randomized: randomize,
            };

            let stego = read_input(&input)?;

            let pb = if output != "-" {
                Some(make_progress("Extracting data..."))
            } else {
                None
            };

            let path_hint = if input == "-" { None } else { Some(input.as_str()) };
            let data = cloak_core::extract(&stego, &passphrase, path_hint, &options)
                .context("extraction failed")?;

            if let Some(pb) = pb {
                pb.finish_with_message("Extraction complete");
            }

            write_output(&output, &data)?;

            if output != "-" {
                println!("Extracted {} bytes to {}", data.len(), output);
            }
        }

        Command::Analyze { input } => {
            let paths = resolve_inputs(&input)?;

            for path in &paths {
                if paths.len() > 1 {
                    println!("--- {} ---", path.display());
                }

                let data = fs::read(path)
                    .with_context(|| format!("failed to read image: {}", path.display()))?;

                let path_str = path.to_string_lossy();
                let format = cloak_core::ImageFormat::detect(&data, Some(&path_str))
                    .context("format detection failed")?;

                let options = cloak_core::EmbedOptions::default();
                let cap = cloak_core::capacity(&data, Some(&path_str), &options)
                    .context("capacity calculation failed")?;

                let img = image::load_from_memory(&data).context("failed to decode image")?;
                let (w, h) = img.dimensions();

                println!("Image:      {}", path.display());
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
                        println!(
                            "  Total pairs: {}  Close pairs: {}",
                            sp.total_pairs, sp.close_pairs
                        );
                        println!("  Estimated rate: {:.4}", sp.estimated_rate);
                    }

                    if let Some(ent) = &analysis.entropy {
                        println!();
                        println!("Shannon Entropy (bits):");
                        println!(
                            "  Red:   {:.4}  Green: {:.4}  Blue:  {:.4}",
                            ent.red, ent.green, ent.blue
                        );
                        println!("  Average: {:.4}", ent.average);
                    }
                }

                if paths.len() > 1 {
                    println!();
                }
            }
        }

        Command::Capacity { input, bit_depth } => {
            let paths = resolve_inputs(&input)?;
            let options = cloak_core::EmbedOptions {
                bit_depth,
                ..Default::default()
            };

            for path in &paths {
                let data = fs::read(path)
                    .with_context(|| format!("failed to read image: {}", path.display()))?;

                let path_str = path.to_string_lossy();
                let cap = cloak_core::capacity(&data, Some(&path_str), &options)
                    .context("capacity calculation failed")?;

                if paths.len() > 1 {
                    println!("{}: {cap} bytes", path.display());
                } else {
                    println!("{cap} bytes");
                }
            }
        }

        Command::Inspect { input } => {
            let data = fs::read(&input)
                .with_context(|| format!("failed to read image: {}", input.display()))?;
            cloak_tui::run_tui(&data, &input.to_string_lossy())
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }

        Command::BatchEmbed {
            input_dir,
            data_dir,
            output_dir,
            passphrase,
            bit_depth,
            randomize,
        } => {
            let passphrase = get_passphrase(passphrase, true)?;
            let options = cloak_core::EmbedOptions {
                bit_depth,
                randomized: randomize,
            };

            fs::create_dir_all(&output_dir)
                .with_context(|| format!("failed to create output dir: {}", output_dir.display()))?;

            let pattern = format!("{}/*", input_dir.display());
            let image_files: Vec<PathBuf> = glob::glob(&pattern)
                .context("invalid input directory")?
                .filter_map(|r| r.ok())
                .filter(|p| p.is_file())
                .collect();

            if image_files.is_empty() {
                bail!("no files found in {}", input_dir.display());
            }

            let pb = ProgressBar::new(image_files.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{bar:40.cyan/blue} {pos}/{len} {msg}")
                    .unwrap(),
            );

            let mut success = 0;
            let mut failed = 0;

            for image_path in &image_files {
                let stem = image_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();
                pb.set_message(stem.to_string());

                let data_pattern = format!("{}/{}.*", data_dir.display(), stem);
                let data_file = glob::glob(&data_pattern)
                    .ok()
                    .and_then(|mut g| g.find_map(|r| r.ok()));

                let data_file = match data_file {
                    Some(f) => f,
                    None => {
                        let exact = data_dir.join(&*stem);
                        if exact.is_file() {
                            exact
                        } else {
                            eprintln!("Warning: no data file for {}, skipping", stem);
                            failed += 1;
                            pb.inc(1);
                            continue;
                        }
                    }
                };

                let cover = match fs::read(image_path) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Error reading {}: {e}", image_path.display());
                        failed += 1;
                        pb.inc(1);
                        continue;
                    }
                };

                let payload = match fs::read(&data_file) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Error reading {}: {e}", data_file.display());
                        failed += 1;
                        pb.inc(1);
                        continue;
                    }
                };

                let path_str = image_path.to_string_lossy();
                let format = match cloak_core::ImageFormat::detect(&cover, Some(&path_str)) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Unsupported format {}: {e}", image_path.display());
                        failed += 1;
                        pb.inc(1);
                        continue;
                    }
                };

                let out_name = format!("{}{}", stem, format.output_format().extension());
                let out_path = output_dir.join(out_name);

                match cloak_core::embed(&cover, &payload, &passphrase, Some(&path_str), &options) {
                    Ok(stego) => {
                        if let Err(e) = fs::write(&out_path, stego) {
                            eprintln!("Error writing {}: {e}", out_path.display());
                            failed += 1;
                        } else {
                            success += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("Embed failed for {}: {e}", image_path.display());
                        failed += 1;
                    }
                }

                pb.inc(1);
            }

            pb.finish_with_message("done");
            println!("Batch embed: {success} succeeded, {failed} failed");
        }

        Command::BatchExtract {
            input_dir,
            output_dir,
            passphrase,
            bit_depth,
            randomize,
        } => {
            let passphrase = get_passphrase(passphrase, false)?;
            let options = cloak_core::EmbedOptions {
                bit_depth,
                randomized: randomize,
            };

            fs::create_dir_all(&output_dir)
                .with_context(|| format!("failed to create output dir: {}", output_dir.display()))?;

            let pattern = format!("{}/*", input_dir.display());
            let stego_files: Vec<PathBuf> = glob::glob(&pattern)
                .context("invalid input directory")?
                .filter_map(|r| r.ok())
                .filter(|p| p.is_file())
                .collect();

            if stego_files.is_empty() {
                bail!("no files found in {}", input_dir.display());
            }

            let pb = ProgressBar::new(stego_files.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{bar:40.cyan/blue} {pos}/{len} {msg}")
                    .unwrap(),
            );

            let mut success = 0;
            let mut failed = 0;

            for stego_path in &stego_files {
                let stem = stego_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();
                pb.set_message(stem.to_string());

                let stego = match fs::read(stego_path) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Error reading {}: {e}", stego_path.display());
                        failed += 1;
                        pb.inc(1);
                        continue;
                    }
                };

                let path_str = stego_path.to_string_lossy();
                let out_path = output_dir.join(format!("{}.dat", stem));

                match cloak_core::extract(&stego, &passphrase, Some(&path_str), &options) {
                    Ok(data) => {
                        if let Err(e) = fs::write(&out_path, data) {
                            eprintln!("Error writing {}: {e}", out_path.display());
                            failed += 1;
                        } else {
                            success += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("Extract failed for {}: {e}", stego_path.display());
                        failed += 1;
                    }
                }

                pb.inc(1);
            }

            pb.finish_with_message("done");
            println!("Batch extract: {success} succeeded, {failed} failed");
        }
    }

    Ok(())
}
