//! The schema describing the entities and relationships in the GraphQL API.

use super::{traits::Many, types::*, D};

/// A US state.
#[derive(Clone, Debug, Class)]
pub struct State {
    /// The 2-letter abbreviation for this state as recognized by the US postal service.
    #[class(primary)]
    abbreviation: String,
    /// The full name of this state.
    name: String,
    /// Bills introduced in this state.
    #[class(plural)]
    bills: Many<D, Bill>,
    /// Legislators serving in this state.
    #[class(plural)]
    legislators: Many<D, Legislator>,
    /// Districts making up this state.
    #[class(plural)]
    districts: Many<D, District>,
}

/// A subdivision of a [`State`] with its own representatives in the state legislature.
#[derive(Clone, Debug, Class)]
pub struct District {
    /// The state containing this district.
    state: State,
    /// The name of the district.
    name: String,
    /// The district ID.
    ///
    /// This is usually a number (like "37" in CA-37), but not always -- some states have an
    /// at-large district with ID "AL".
    id: String,
    /// Representatives of this district in the state legislature.
    #[class(plural)]
    representatives: Many<D, Legislator>,
}

/// A piece of legislation.
#[derive(Clone, Debug, Class)]
pub struct Bill {
    /// The name of the bill.
    ///
    /// This is frequently a combination of a chamber identifier (like "SB" for senate bill) and a
    /// bill number, as in "SB-999".
    name: String,
    /// A short summary of the effects of the bill.
    ///
    /// The summary may be provided by the sponsors of the bill or a non-partisan office, but it is
    /// important to remember that it is just some person's or group of people's interpretation of
    /// the bill. Unlike the text of the bill itself, it is not legally binding.
    summary: String,
    /// The state in which this bill was introduced.
    state: State,
    /// Legislators sponsoring the bill.
    #[class(plural)]
    sponsors: Many<D, Legislator>,
    /// Issues that the bill relates to.
    #[class(plural)]
    issues: Many<D, Issue>,
}

/// A state lawmaker.
#[derive(Clone, Debug, Class)]
pub struct Legislator {
    /// The legislator's legal name.
    name: String,
    /// The state in which the legislator serves.
    state: State,
    /// The district in `state` which the legislator represents.
    district: District,
    /// The legislator's political party.
    party: Party,
    /// Bills the legislator has sponsored.
    #[class(plural)]
    sponsored_bills: Many<D, Bill>,
}

/// A political party.
#[derive(Clone, Debug, Class)]
#[class(plural(Parties))]
pub struct Party {
    /// A shortened form of the party's name.
    ///
    /// This is frequently a single letter as in "R" (for Republican) or "D" (for Democrat). For
    /// obscure parties, though, this may be as long as the full name of the party.
    #[class(primary)]
    abbreviation: String,
    /// The full name of the party.
    name: String,
    /// State lawmakers who are members of this party.
    #[class(plural)]
    members: Many<D, Legislator>,
}

/// A political issue.
#[derive(Clone, Debug, Class)]
pub struct Issue {
    /// A short name for the issue.
    name: String,
    /// Bills pertaining to this issue.
    #[class(plural)]
    bills: Many<D, Bill>,
}

/// Entrypoint for read-only GraphQL queries.
#[derive(Clone, Copy, Debug, Query)]
#[query(class(bills: Bill))]
#[query(class(legislators: Legislator))]
#[query(class(states: State))]
#[query(class(districts: District))]
#[query(class(parties: Party))]
#[query(class(issues: Issue))]
pub struct Query;

/// Create the schema for the GraphQL API.
pub fn generate() -> Schema<Query, EmptyMutation, EmptySubscription> {
    Schema::build(Query, EmptyMutation, EmptySubscription).finish()
}
