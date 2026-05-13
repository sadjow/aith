mod cli;
mod paths;
mod profiles;
mod tools;

fn main() -> anyhow::Result<()> {
    cli::run()
}
