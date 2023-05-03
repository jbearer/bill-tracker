//! The Postgres database used by the bill tracker application.

use super::{
    legiscan::{Bill, Dataset, DatasetMetadata, Legiscan, Party, Person, State},
    schema,
};
use anyhow::Error;
use clap::Args;
use futures::future::{try_join, try_join_all};
use relational_graphql::{
    graphql::{
        backend::{DataSource, PageRequest},
        type_system::{
            i32_scalar::Predicate as I32Predicate, Id, Resource, StringPredicate, Value,
        },
    },
    sql::{db::postgres, PostgresDataSource},
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use strum::IntoEnumIterator;
use surf::Url;

/// Database connection options.
#[derive(Clone, Debug, Args)]
pub struct Options {
    /// URL for connecting to the Postgres database.
    #[clap(
        long,
        env = "BILL_TRACKER_DB_URL",
        default_value = "http://localhost:5432"
    )]
    pub db_url: Url,

    /// User as which to connect to the database.
    #[clap(long, env = "BILL_TRACKER_DB_USER", default_value = "postgres")]
    pub db_user: String,

    /// Password for connecting to the Postgres database.
    #[clap(long, env = "BILL_TRACKER_DB_PASSWORD", default_value = "password")]
    pub db_password: String,
}

impl Options {
    /// Connect to the databse.
    pub async fn connect(&self) -> Result<Connection, Error> {
        let mut config = postgres::Config::default();
        let host = self
            .db_url
            .host()
            .ok_or_else(|| Error::msg(format!("URL {} has no hostname", self.db_url)))?
            .to_string();
        config
            .user(&self.db_user)
            .password(&self.db_password)
            .host(&host);
        if let Some(port) = self.db_url.port() {
            config.port(port);
        }
        Ok(postgres::Connection::new(config).await?.into())
    }
}

/// A connection to the database.
pub type Connection = PostgresDataSource;

/// Perform one-time setup of the database.
///
/// This will create the necessary tables and relations, and populate static data like state and
/// party information.
pub async fn setup(conn: &mut Connection) -> Result<(), Error> {
    schema::Query::register(conn).await?;

    // Populate states.
    let states = State::iter().map(|state| schema::state::StateInput {
        abbreviation: state.to_string(),
        name: state.name().into(),
    });
    conn.insert::<schema::State, _>(states).await?;

    // Populate political parties.
    let parties = Party::iter().map(|party| schema::party::PartyInput {
        abbreviation: party.abbreviation().into(),
        name: party.to_string(),
    });
    conn.insert::<schema::Party, _>(parties).await?;

    Ok(())
}

/// Update information in the database based on the latest bulk download from Legiscan.
///
/// If `out` is provided, the data pulled to legiscan will be saved to disk as well as persisted in
/// the database.
pub async fn update<L: Legiscan, P: AsRef<Path>>(
    conn: &mut Connection,
    legiscan: &L,
    datasets: Vec<L::DatasetMetadata>,
    out: Option<P>,
) -> Result<(), Error> {
    for meta in &datasets {
        let dataset = legiscan.load_dataset(meta).await?;
        tracing::info!("pulling dataset {}", meta.id());

        if let Some(out) = &out {
            dataset.extract(out.as_ref())?;
        }

        // First, just query existing information to figure out which things need to be inserted or
        // updated. These queries can be done in parallel.
        let read_conn = &conn;
        let bill_actions = dataset.bills().map(|bill| async move {
            tracing::info!("bill {} {} - {}", bill.state(), bill.name(), bill.title());

            // Check if this bill already exists and, if it does, whether it needs to be updated.
            let actions = match find_bill(read_conn, bill.id()).await? {
                Some(existing) => {
                    if existing.legiscan_hash == bill.hash() {
                        // This bill is unchanged, nothing to do.
                        tracing::info!("bill {} is up-to-date", bill.id());
                        vec![]
                    } else {
                        // TODO implement updates for existing data.
                        tracing::warn!(
                            "not yet implemented: updates; bill {} will be out of date",
                            bill.id()
                        );
                        vec![]
                    }
                }
                None => {
                    // The people that this bill depends on will all be inserted from the people
                    // section of this dataset. The issues, on the other hand, need to be created
                    // now if they don't exist already.
                    let mut actions =
                        try_join_all(bill.issues().into_iter().map(|issue| async move {
                            if find_issue(read_conn, issue.clone()).await?.is_none() {
                                Ok(Some(Action::InsertIssue(issue)))
                            } else {
                                Ok::<_, Error>(None)
                            }
                        }))
                        .await?
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    actions.extend([
                        Action::InsertBill(schema::bill::BillInput {
                            legiscan_id: bill.id(),
                            legiscan_hash: bill.hash(),
                            name: bill.name(),
                            title: bill.title(),
                            summary: bill.summary(),
                            state: bill.state().id().into(),
                        }),
                        Action::LinkBill {
                            bill: bill.id(),
                            sponsors: bill.sponsors(),
                            issues: bill.issues(),
                        },
                    ]);
                    actions
                }
            };

            Ok::<_, Error>(actions)
        });
        let people_actions = dataset.people().map(|person| async move {
            tracing::info!(
                "person {} ({} - {})",
                person.name(),
                person.party().abbreviation(),
                person.district()
            );

            // Check if this person already exists and, if it does, whether it needs to be updated.
            let actions = match find_person(read_conn, person.id()).await? {
                Some(existing) => {
                    if existing.legiscan_hash == person.hash() {
                        // This person is unchanged, nothing to do.
                        tracing::info!("person {} is up-to-date", person.id());
                        vec![]
                    } else {
                        // TODO implement updates for existing data.
                        tracing::warn!(
                            "not yet implemented: updates; person {} will be out of date",
                            person.id()
                        );
                        vec![]
                    }
                }
                None => {
                    let name = person.name();
                    let state = person.state();

                    // Look up the district; if it doesn't exist, we need to insert it.
                    let district_name = person.district();
                    let district = find_district(read_conn, state, district_name.clone()).await?;
                    let build_person = move |district| schema::legislator::LegislatorInput {
                        legiscan_id: person.id(),
                        legiscan_hash: person.hash(),
                        first_name: name.first,
                        middle_name: name.middle,
                        last_name: name.last,
                        district,
                        party: person.party().id().into(),
                    };
                    match district {
                        Some(district) => vec![Action::InsertPerson(build_person(district.id))],
                        None => vec![
                            Action::InsertDistrict(InsertDistrict {
                                name: district_name.clone(),
                                state,
                            }),
                            Action::BuildPerson {
                                district: district_name,
                                build: Box::new(build_person),
                            },
                        ],
                    }
                }
            };

            Ok::<_, Error>(actions)
        });
        let (bill_actions, people_actions) =
            try_join(try_join_all(bill_actions), try_join_all(people_actions)).await?;
        let actions = bill_actions.into_iter().chain(people_actions).flatten();

        // Break the actions into batches, each of which can be parallelized:
        // 1. Insert all districts, and create a map from district names to IDs. This will be
        //    necessary for building legislator objects to insert later.
        let mut insert_districts: HashSet<InsertDistrict> = Default::default();
        // 2. Insert all people, lazily building them from the newly inserted district IDs if
        //    necessary.
        let mut insert_people: Vec<schema::legislator::LegislatorInput> = Default::default();
        let mut build_people: Vec<(String, PersonBuilder)> = Default::default();
        // 3. Insert all bills.
        let mut insert_bills: Vec<schema::bill::BillInput> = Default::default();
        // 4. Insert all issues.
        let mut insert_issues: HashSet<String> = Default::default();
        // 5. Add relations between bills and their sponsors.
        let mut bill_sponsors: Vec<(String, String)> = Default::default();
        // 6. Add relations between bills and their issues.
        let mut bill_issues: Vec<(String, String)> = Default::default();
        for action in actions {
            match action {
                Action::InsertDistrict(district) => {
                    insert_districts.insert(district);
                }
                Action::InsertPerson(person) => {
                    insert_people.push(person);
                }
                Action::BuildPerson { district, build } => {
                    build_people.push((district, build));
                }
                Action::InsertBill(bill) => {
                    insert_bills.push(bill);
                }
                Action::InsertIssue(name) => {
                    insert_issues.insert(name);
                }
                Action::LinkBill {
                    bill,
                    sponsors,
                    issues,
                } => {
                    for sponsor in sponsors {
                        bill_sponsors.push((bill.clone(), sponsor));
                    }
                    for issue in issues {
                        bill_issues.push((bill.clone(), issue));
                    }
                }
            }
        }

        // Now, in series, execute each batch of actions.
        conn.insert::<schema::District, _>(
            insert_districts
                .iter()
                .cloned()
                .map(schema::district::DistrictInput::from),
        )
        .await?;
        let read_conn = &conn;

        // Get the district IDs we just inserted, indexing them by name.
        let district_ids = try_join_all(insert_districts.into_iter().map(|district| async move {
            match find_district(read_conn, district.state, district.name.clone()).await? {
                Some(found) => Ok((district.name, found.id)),
                None => Err(Error::msg(format!(
                    "ICE: expected to find district {} {} after inserting it",
                    district.state, &district.name
                ))),
            }
        }))
        .await?
        .into_iter()
        .collect::<HashMap<_, _>>();

        // Build and insert people based on the district IDs.
        conn.insert::<schema::Legislator, _>(
            insert_people.into_iter().chain(
                build_people
                    .into_iter()
                    .map(|(district, build)| {
                        let district = district_ids.get(&district).ok_or_else(|| {
                            Error::msg(format!(
                                "ICE: expected to find district {district} after inserting it"
                            ))
                        })?;
                        Ok(build(*district))
                    })
                    .collect::<Result<Vec<_>, Error>>()?,
            ),
        )
        .await?;

        // Insert issues.
        conn.insert::<schema::Issue, _>(
            insert_issues
                .into_iter()
                .map(|name| schema::issue::IssueInput { name }),
        )
        .await?;

        // Insert bills.
        conn.insert::<schema::Bill, _>(insert_bills).await?;

        // Finally, add relations between the newly inserted data (bills to sponsors and issues).
        let read_conn = &conn;
        let (bill_sponsors, bill_issues) = try_join(
            try_join_all(
                bill_sponsors
                    .into_iter()
                    .map(|(bill_id, sponsor_id)| async move {
                        let bill = match find_bill(read_conn, bill_id.clone()).await? {
                            Some(found) => found.id,
                            None => {
                                return Err(Error::msg(format!(
                                    "ICE: expected to find bill {bill_id} after inserting it"
                                )))
                            }
                        };
                        let sponsor = match find_person(read_conn, sponsor_id.clone()).await? {
                            Some(found) => found.id,
                            None => {
                                return Err(Error::msg(format!(
                                    "ICE: expected to find sponsor {sponsor_id} after inserting it"
                                )))
                            }
                        };
                        Ok((bill, sponsor))
                    }),
            ),
            try_join_all(
                bill_issues
                    .into_iter()
                    .map(|(bill_id, issue_name)| async move {
                        let bill = match find_bill(read_conn, bill_id.clone()).await? {
                            Some(found) => found.id,
                            None => {
                                return Err(Error::msg(format!(
                                    "ICE: expected to find bill {bill_id} after inserting it"
                                )))
                            }
                        };
                        let issue = match find_issue(read_conn, issue_name.clone()).await? {
                            Some(found) => found.id,
                            None => {
                                return Err(Error::msg(format!(
                                    "ICE: expected to find issue {issue_name} after inserting it"
                                )))
                            }
                        };
                        Ok((bill, issue))
                    }),
            ),
        )
        .await?;
        conn.populate_relation::<schema::bill::fields::Sponsors, _>(bill_sponsors)
            .await?;
        conn.populate_relation::<schema::bill::fields::Issues, _>(bill_issues)
            .await?;
    }

    Ok(())
}

/// Actions to perform when updating the database.
enum Action {
    InsertDistrict(InsertDistrict),
    InsertBill(schema::bill::BillInput),
    InsertIssue(String),
    LinkBill {
        bill: String,
        sponsors: Vec<String>,
        issues: Vec<String>,
    },
    InsertPerson(schema::legislator::LegislatorInput),
    BuildPerson {
        district: String,
        build: PersonBuilder,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InsertDistrict {
    state: State,
    name: String,
}

type PersonBuilder = Box<dyn Send + FnOnce(Id) -> schema::legislator::LegislatorInput>;

impl From<InsertDistrict> for schema::district::DistrictInput {
    fn from(district: InsertDistrict) -> Self {
        Self {
            state: district.state.id().into(),
            name: district.name,
        }
    }
}

async fn find_bill(conn: &Connection, id: String) -> Result<Option<schema::Bill>, Error> {
    find_one(
        conn,
        schema::Bill::has()
            .legiscan_id(StringPredicate::Is(Value::Lit(id)))
            .into(),
    )
    .await
}

async fn find_person(conn: &Connection, id: String) -> Result<Option<schema::Legislator>, Error> {
    find_one(
        conn,
        schema::Legislator::has()
            .legiscan_id(StringPredicate::Is(Value::Lit(id)))
            .into(),
    )
    .await
}

async fn find_issue(conn: &Connection, id: String) -> Result<Option<schema::Issue>, Error> {
    find_one(
        conn,
        schema::Issue::has()
            .name(StringPredicate::Is(Value::Lit(id)))
            .into(),
    )
    .await
}

async fn find_district(
    conn: &Connection,
    state: State,
    name: String,
) -> Result<Option<schema::District>, Error> {
    find_one(
        conn,
        schema::District::has()
            .state(
                schema::State::has()
                    .id(I32Predicate::Is(Value::Lit(state.id().into())))
                    .into(),
            )
            .name(StringPredicate::Is(Value::Lit(name)))
            .into(),
    )
    .await
}

async fn find_one<T: Resource>(
    conn: &Connection,
    filter: T::Predicate,
) -> Result<Option<T>, Error> {
    let results = conn.query::<T>(Some(filter)).await?;
    let mut page = conn
        .load_page(
            &results,
            PageRequest {
                first: Some(1),
                ..Default::default()
            },
        )
        .await?;
    Ok(if page.is_empty() {
        None
    } else {
        Some(page.remove(0).into_node())
    })
}
