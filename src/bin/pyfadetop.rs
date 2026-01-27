use anyhow::Error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    fadetop::cli::run().await
}
