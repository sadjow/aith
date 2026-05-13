mod cli;
mod tools;

fn main() -> anyhow::Result<()> {
    cli::run()
}
