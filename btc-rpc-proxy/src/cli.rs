use clap::Parser;

#[derive(Debug, Parser)]
#[command(version)]
pub struct Cli {
  #[command(flatten)]
  pub run: RunCmd,
}

#[derive(Debug, clap::Args)]
pub struct RunCmd {
  #[arg(
    long,
    help = "The target host to where forwarding requests. MUST BE HTTPS"
  )]
  pub forward: String,
  #[arg(
    long,
    default_value = "0.0.0.0",
    help = "Binding address, default 0.0.0.0"
  )]
  pub addr: std::net::IpAddr,
  #[arg(
    long,
    short,
    default_value = "8000",
    help = "Binding port, default 8000"
  )]
  pub port: u16,
}
