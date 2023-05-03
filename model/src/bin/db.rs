use clap::Parser;
use model::{
    db, init_logging,
    legiscan::{Client, Legiscan, LocalClient, State},
};
use std::path::PathBuf;

/// Pull the latest data from Legiscan and update the local database.
#[derive(Parser)]
enum Command {
    /// Perform one-time setup of the database.
    Init {
        #[clap(flatten)]
        db: db::Options,
    },
    /// Update the information in the database based on the latest bulk download from Legiscan.
    Pull {
        /// The Legiscan API key to connect with.
        #[clap(short = 'k', long, env = "LEGISCAN_API_KEY")]
        api_key: String,

        /// Only pull data for STATE.
        #[clap(short, long, env = "LEGISCAN_STATE", name = "STATE")]
        state: Option<State>,

        /// Only pull data for YEAR.
        #[clap(short, long, env = "LEGISCAN_YEAR", name = "YEAR")]
        year: Option<u16>,

        /// Write raw datasets to DIR.
        #[clap(short, long, env = "LEGISCAN_OUT", name = "DIR")]
        out: Option<PathBuf>,

        #[clap(flatten)]
        db: db::Options,
    },
    /// Update information in the database based on datasets saved in local storage.
    Read {
        /// The path to the directory containing the local datasets.
        ///
        /// This should be a directory with the structure
        ///
        ///     DIR/
        ///         <state>/
        ///             <session>/
        ///                 hash.md5
        ///                 bill/
        ///                 people/
        #[clap(short, long, env = "LEGISCAN_DATA_DIR", name = "DIR")]
        dir: PathBuf,

        /// Only pull data for STATE.
        #[clap(short, long, env = "LEGISCAN_STATE", name = "STATE")]
        state: Option<State>,

        /// Only pull data for YEAR.
        #[clap(short, long, env = "LEGISCAN_YEAR", name = "YEAR")]
        year: Option<u16>,

        /// Write raw datasets to DIR.
        #[clap(short, long, env = "LEGISCAN_OUT", name = "DIR")]
        out: Option<PathBuf>,

        #[clap(flatten)]
        db: db::Options,
    },
}

#[async_std::main]
async fn main() -> Result<(), anyhow::Error> {
    init_logging();

    match Command::parse() {
        Command::Init { db } => {
            let mut conn = db.connect().await?;
            db::setup(&mut conn).await?;
        }
        Command::Pull {
            api_key,
            state,
            year,
            out,
            db,
        } => {
            let client = Client::new(api_key);
            let datasets = client.list_datasets(state, year).await?;
            tracing::info!("{} datasets available", datasets.len());

            let mut conn = db.connect().await?;
            db::update(&mut conn, &client, datasets, out.as_ref()).await?;
        }
        Command::Read {
            dir,
            state,
            year,
            out,
            db,
        } => {
            let client = LocalClient::open(dir);
            let datasets = client.list_datasets(state, year).await?;
            tracing::info!("{} datasets available", datasets.len());

            let mut conn = db.connect().await?;
            db::update(&mut conn, &client, datasets, out.as_ref()).await?;
        }
    }

    Ok(())
}
