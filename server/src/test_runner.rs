#![cfg(test)]

//! This module runs test cases defined in `db/test/cases`.
//!
//! Each test case consists of two files, named in terms of `<name>`, the name of the test case:
//! * `<name>.graphql`: a GraphQL object to query for
//! * `<name>.json`: the expected JSON response
//!
//! This runner will start a server and scan that directory for all such pairs of files, executing
//! each query and making sure that the response matches the expected response. Before comparing
//! the expected and actual responses, the test runner will sort any array named "edges", to avoid
//! dependency on implementation-defined ordering.
//!
//! To run these tests, first make sure the test database is up and running, if you haven't already:
//! ```ignore
//! bin/start-test-db
//! cargo run --release --bin create-test-db
//! ```
//! Then use `cargo test --release -p server` to run the tests.

use super::Options;
use ansi_term::Color;
use anyhow::Error;
use async_std::task::{sleep, spawn};
use futures::future::join_all;
use model::db;
use portpicker::pick_unused_port;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File};
use std::path::Path;
use std::time::Duration;
use surf::{http::StatusCode, Client};

#[async_std::test]
async fn graphql_api_test_cases() -> Result<(), Error> {
    relational_graphql::init_logging();

    // Discover test cases.
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let test_cases = workspace
        .join("db/test/cases")
        .read_dir()?
        .filter_map(|dirent| {
            let path = dirent.unwrap().path();
            if path.extension()?.to_str().unwrap() == "graphql" {
                Some(TestCase::new(&path).unwrap())
            } else {
                None
            }
        });

    // Start a GraphQL server.
    let port = pick_unused_port().unwrap();
    let opt = Options {
        port,
        db: db::Options::test(),
    };
    spawn(async move {
        opt.serve().await.unwrap();
        tracing::warn!("server exited");
    });

    // Connect a client.
    let client: Client = surf::Config::default()
        .set_base_url(format!("http://localhost:{port}").parse().unwrap())
        .try_into()
        .unwrap();
    // Wait for the server to come up.
    wait_for_server(&client).await?;

    let results = join_all(test_cases.map(|test| test.run(client.clone()))).await;
    for result in &results {
        println!("{}", result);
    }
    if results.iter().any(TestResult::failed) {
        Err(Error::msg(format!("{}", Color::Red.paint("tests failed"))))
    } else {
        println!("All test cases passed.");
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct TestCase {
    name: OsString,
    query: String,
    response: Value,
}

impl TestCase {
    fn new(query_path: impl AsRef<Path>) -> Result<Self, Error> {
        let query_path = query_path.as_ref();
        let name = query_path.file_stem().unwrap();
        let query_bytes = fs::read(query_path)?;
        let query = std::str::from_utf8(&query_bytes)?;
        let response_path = query_path.with_extension("json");
        let mut response = serde_json::from_reader(File::open(response_path)?)?;
        normalize_response(&mut response);
        Ok(Self {
            name: name.into(),
            query: query.into(),
            response,
        })
    }

    async fn run(self, client: Client) -> TestResult {
        TestResult {
            name: self.name,
            failure: Self::do_test(client, self.query, self.response).await.err(),
        }
    }

    async fn do_test(client: Client, query: String, expected_response: Value) -> Result<(), Error> {
        // Make the GraphQL request.
        let mut res = client
            .post("/graphql")
            .body_json(&json!({ "query": query }))
            .map_err(Error::msg)?
            .send()
            .await
            .map_err(Error::msg)?;
        if res.status() != StatusCode::Ok {
            return Err(Error::msg(format!(
                "query failed with status {}",
                res.status()
            )));
        }

        // Parse and normalize the response.
        let mut response: Value = res
            .body_json()
            .await
            .map_err(|err| Error::msg(format!("cannot parse reponse body as JSON: {err}")))?;
        normalize_response(&mut response);

        // Extract GraphQL errors.
        for error in response
            .get("errors")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
        {
            tracing::error!("GraphQL error: {error}");
        }

        // Extract GraphQL data.
        let data = response
            .get("data")
            .ok_or_else(|| Error::msg(format!("response is missing data: {response}")))?;
        if *data != expected_response {
            Err(Error::msg(format!(
                "expected response:\n{expected_response}\nactual response:\n{data}"
            )))
        } else {
            Ok(())
        }
    }
}

struct TestResult {
    name: OsString,
    failure: Option<anyhow::Error>,
}

impl TestResult {
    fn failed(&self) -> bool {
        self.failure.is_some()
    }
}

impl Display for TestResult {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}...", self.name.to_string_lossy())?;
        if let Some(err) = &self.failure {
            writeln!(f, "{}", Color::Red.paint("FAILED"))?;
            write!(f, "{err}")?;
        } else {
            write!(f, "{}", Color::Green.paint("OK"))?;
        }
        Ok(())
    }
}

async fn wait_for_server(client: &Client) -> Result<(), Error> {
    const MAX_CONNECT_RETRIES: usize = 60;

    for _ in 0..MAX_CONNECT_RETRIES {
        match client.connect("/").await {
            Ok(_) => return Ok(()),
            Err(err) => {
                tracing::warn!("waiting for server to start: {err}");
                sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Err(Error::msg("timed out waiting for server"))
}

fn normalize_response(res: &mut Value) {
    // Normalize all children.
    if let Some(obj) = res.as_object_mut() {
        for (key, val) in obj.iter_mut() {
            normalize_response(val);

            // If this value is an `edges` array, sort it.
            if key == "edges" {
                if let Some(arr) = val.as_array_mut() {
                    arr.sort_by_key(|val| val.to_string());
                }
            }
        }
    } else if let Some(arr) = res.as_array_mut() {
        for val in arr {
            normalize_response(val);
        }
    }
}
