use async_graphql_tide::graphql;
use clap::Parser;
use model::{db, schema};

/// Start the bill tracker server.
#[derive(Clone, Debug, Parser)]
struct Options {
    /// The port where the app should be served.
    #[clap(short, long, env = "BILL_TRACKER_PORT", default_value = "80")]
    port: u16,

    #[clap(flatten)]
    db: db::Options,
}

impl Options {
    async fn serve(&self) -> tide::Result<()> {
        let mut app = tide::new();
        app.at("/graphql")
            .all(graphql(schema::executor(&self.db).await?));
        app.listen(format!("0.0.0.0:{}", self.port)).await?;
        Ok(())
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    relational_graphql::init_logging();
    Options::parse().serve().await
}

mod test_runner;
