use clap::Parser;
use model::{
    db, init_logging,
    legiscan::{Legiscan, LocalClient},
};
use std::path::PathBuf;
use surf::Url;

/// Set up a test database and seed it with a small amount of data for easy testing.
#[derive(Parser)]
struct Options {
    /// The path to the directory containing the test data.
    ///
    /// This should be a directory with the structure
    ///
    ///     DIR/
    ///         <state>/
    ///             <session>/
    ///                 hash.md5
    ///                 bill/
    ///                 people/
    #[clap(
        short,
        long,
        env = "BILL_TRACKER_TEST_DATA_DIR",
        name = "DIR",
        default_value = "db/test/data"
    )]
    dir: PathBuf,

    /// URL for connecting to the Postgres database.
    #[clap(
        long,
        env = "BILL_TRACKER_TEST_DB_URL",
        default_value = "http://localhost:5433"
    )]
    db_url: Url,

    /// User as which to connect to the database.
    #[clap(long, env = "BILL_TRACKER_TEST_DB_USER", default_value = "postgres")]
    db_user: String,

    /// Password for connecting to the Postgres database.
    #[clap(
        long,
        env = "BILL_TRACKER_TEST_DB_PASSWORD",
        default_value = "password"
    )]
    db_password: String,
}

#[async_std::main]
async fn main() -> Result<(), anyhow::Error> {
    init_logging();

    let opt = Options::parse();
    let db_opt = db::Options {
        db_url: opt.db_url,
        db_user: opt.db_user,
        db_password: opt.db_password,
    };
    let mut conn = db_opt.connect().await?;

    // Set up the schema.
    db::setup(&mut conn).await?;

    // Insert test data.
    let client = LocalClient::open(opt.dir);
    let datasets = client.list_datasets(None, None).await?;
    db::update::<_, PathBuf>(&mut conn, &client, datasets, None).await?;

    Ok(())
}
