//! The schema describing the entities and relationships in the GraphQL API.

use super::prelude::*;

/// A US state.
#[derive(Clone, Debug, Resource)]
pub struct State {
    id: Id,
    /// The 2-letter abbreviation for this state as recognized by the US postal service.
    #[resource(primary)]
    abbreviation: String,
    /// The full name of this state.
    name: String,
    /// Bills introduced in this state.
    bills: BelongsTo<Bill>,
    /// Legislators serving in this state.
    legislators: BelongsTo<Legislator>,
    /// Districts making up this state.
    districts: BelongsTo<District>,
}

/// A subdivision of a [`State`] with its own representatives in the state legislature.
#[derive(Clone, Debug, Resource)]
pub struct District {
    id: Id,
    /// The state containing this district.
    state: State,
    /// The name of the district.
    ///
    /// This is usually a number (like "37" in CA-37), but not always -- some states have an
    /// at-large district with ID "AL".
    name: String,
    /// The locale of the district.
    ///
    /// This is usually the name of the most populous city or county in the district. It is less
    /// precise but easier to understand than the `name`.
    locale: String,
    /// Representatives of this district in the state legislature.
    representatives: BelongsTo<Legislator>,
}

/// A piece of legislation.
#[derive(Clone, Debug, Resource)]
pub struct Bill {
    id: Id,
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
    #[resource(inverse(sponsored_bills))]
    sponsors: Many<Legislator>,
    /// Issues that the bill relates to.
    issues: Many<Issue>,
}

/// A state lawmaker.
#[derive(Clone, Debug, Resource)]
pub struct Legislator {
    id: Id,
    /// The legislator's legal name.
    name: String,
    /// The state in which the legislator serves.
    state: State,
    /// The district in `state` which the legislator represents.
    district: District,
    /// The legislator's political party.
    party: Party,
    /// Bills the legislator has sponsored.
    #[resource(inverse(sponsors))]
    sponsored_bills: Many<Bill>,
}

/// A political party.
#[derive(Clone, Debug, Resource)]
#[resource(plural(Parties))]
pub struct Party {
    id: Id,
    /// A shortened form of the party's name.
    ///
    /// This is frequently a single letter as in "R" (for Republican) or "D" (for Democrat). For
    /// obscure parties, though, this may be as long as the full name of the party.
    #[resource(primary)]
    abbreviation: String,
    /// The full name of the party.
    name: String,
    /// State lawmakers who are members of this party.
    members: BelongsTo<Legislator>,
}

/// A political issue.
#[derive(Clone, Debug, Resource)]
pub struct Issue {
    id: Id,
    /// A short name for the issue.
    name: String,
    /// Bills pertaining to this issue.
    bills: Many<Bill>,
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::sql::{db::mock::Connection, SqlDataSource};

    #[async_std::test]
    async fn test_register_schema() {
        let mut db = SqlDataSource::from(Connection::create());
        Query::register(&mut db).await.unwrap();

        // As a smoketest, make sure all the resources were registered.
        let mut resources = db.inner().schema().await.into_keys().collect::<Vec<_>>();
        resources.sort();
        assert_eq!(
            resources,
            [
                "bills",
                "districts",
                "issues",
                "legislators",
                "parties",
                "states",
            ]
        );
    }
}
