#![feature(str_split_whitespace_remainder)]

mod cli;
mod engine;
mod shogi;

fn main() -> std::io::Result<()> {
    let cli_options = cli::parse();
    dbg!(&cli_options);
    Ok(())
}
