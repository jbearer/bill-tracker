//! The schema describing the entities and relationships in the GraphQL API.

use relational_graphql::prelude::*;

/// A US state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Resource)]
pub struct State {
    pub id: Id,
    /// The 2-letter abbreviation for this state as recognized by the US postal service.
    #[resource(primary)]
    pub abbreviation: String,
    /// The full name of this state.
    pub name: String,
    /// Bills introduced in this state.
    pub bills: BelongsTo<Bill>,
    /// Districts making up this state.
    pub districts: BelongsTo<District>,
}

/// A subdivision of a [`State`] with its own representatives in the state legislature.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Resource)]
pub struct District {
    pub id: Id,
    /// The state containing this district.
    pub state: State,
    /// The name of the district.
    ///
    /// This is usually a number (like "37" in CA-37), but not always -- some states have an
    /// at-large district with ID "AL".
    pub name: String,
    /// Representatives of this district in the state legislature.
    pub representatives: BelongsTo<Legislator>,
}

/// A piece of legislation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Resource)]
pub struct Bill {
    pub id: Id,
    /// The ID of this bill in Legiscan.
    pub legiscan_id: String,
    /// The hash of this bill in Legiscan, for change detection.
    pub legiscan_hash: String,
    /// The name of the bill.
    ///
    /// This is frequently a combination of a chamber identifier (like "SB" for senate bill) and a
    /// bill number, as in "SB-999". It is always unique within a session.
    pub name: String,
    /// A readable title for the bill.
    ///
    /// Compared to `name`, `title` is similarly brief, more descriptive, but less precise (it is
    /// freeform language, and not guranteed unique within a session, although in practice it
    /// usually will be).
    pub title: String,
    /// A short summary of the effects of the bill.
    ///
    /// The summary may be provided by the sponsors of the bill or a non-partisan office, but it is
    /// important to remember that it is just some person's or group of people's interpretation of
    /// the bill. Unlike the text of the bill itself, it is not legally binding.
    pub summary: String,
    /// The state in which this bill was introduced.
    pub state: State,
    /// Legislators sponsoring the bill.
    #[resource(inverse(sponsored_bills))]
    pub sponsors: Many<Legislator>,
    /// Issues that the bill relates to.
    pub issues: Many<Issue>,
}

/// A state lawmaker.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Resource)]
pub struct Legislator {
    pub id: Id,
    /// The ID of this legislator in Legiscan.
    pub legiscan_id: String,
    /// The hash of this legislator in Legiscan, for change detection.
    pub legiscan_hash: String,
    /// The legislator's first name.
    pub first_name: String,
    /// The legislator's middle name.
    pub middle_name: String,
    /// The legislator's last name.
    pub last_name: String,
    /// The district in `state` which the legislator represents.
    pub district: District,
    /// The legislator's political party.
    pub party: Party,
    /// Bills the legislator has sponsored.
    #[resource(inverse(sponsors))]
    pub sponsored_bills: Many<Bill>,
}

/// A political party.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Resource)]
#[resource(plural(Parties))]
pub struct Party {
    pub id: Id,
    /// A shortened form of the party's name.
    ///
    /// This is frequently a single letter as in "R" (for Republican) or "D" (for Democrat). For
    /// obscure parties, though, this may be as long as the full name of the party.
    #[resource(primary)]
    pub abbreviation: String,
    /// The full name of the party.
    pub name: String,
    /// State lawmakers who are members of this party.
    pub members: BelongsTo<Legislator>,
}

/// A political issue.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Resource)]
pub struct Issue {
    pub id: Id,
    /// The ID of this issue in Legiscan.
    pub legiscan_id: String,
    /// A short name for the issue.
    pub name: String,
    /// Bills pertaining to this issue.
    pub bills: Many<Bill>,
}

/// Entrypoint for read-only GraphQL queries.
#[derive(Clone, Copy, Debug, Query)]
#[query(resource(bills: Bill))]
#[query(resource(legislators: Legislator))]
#[query(resource(states: State))]
#[query(resource(districts: District))]
#[query(resource(parties: Party))]
#[query(resource(issues: Issue))]
pub struct Query;

/// Create the schema for the GraphQL API.
pub fn generate() -> Schema<Query, EmptyMutation, EmptySubscription> {
    Schema::build(Query, EmptyMutation, EmptySubscription).finish()
}
