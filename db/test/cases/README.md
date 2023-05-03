# GraphQL API Test Cases

This directory contains test cases for the bill tracker GraphQL API, assuming the backend is serving
the data in [the test data set](../data). Each test case consists of two files, named in terms of
`<name>`, the name of the test case:
* `<name>.graphql`: a GraphQL object to query for
* `<name>.json`: the expected JSON response

A test runner is included as a unit test in the `server` crate. It will scan this directory for all
such pairs of files and execute each query, checking that the response matches the expected
response. Before comparing the expected and actual responses, the test runner will sort any array
named "edges", to avoid dependency on implementation-defined ordering.

To run it from the workspace root, first make sure the test database is up and running, if you
haven't already:
```
bin/start-test-db
cargo run --release --bin create-test-db
```
Then use `cargo test --release -p server` to run the tests.
