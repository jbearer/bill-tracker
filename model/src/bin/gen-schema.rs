use clap::Parser;

/// Generate the GraphQL schema for the Rust data model.
#[derive(Parser)]
struct Options;

fn main() {
    Options::parse();
    println!("{}", model::graphql::schema::generate().sdl());
}
