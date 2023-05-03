//! Facilities for pulling data from Legiscan.

use anyhow::Error;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::path::Path;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

mod client;
mod local;

pub use client::Client;
pub use local::LocalClient;

/// A Legiscan client.
#[async_trait]
pub trait Legiscan {
    /// A dataset, containing bulk information about many people and bills.
    type Dataset: Dataset;

    /// Succinct metadata about a dataset, without the full contents.
    type DatasetMetadata: DatasetMetadata;

    /// List available datasets, optionally filtering by state and year.
    async fn list_datasets(
        &self,
        state: Option<State>,
        year: Option<u16>,
    ) -> Result<Vec<Self::DatasetMetadata>, Error>;

    /// Load a dataset, including all bills, people, and votes involved.
    async fn load_dataset(&self, dataset: &Self::DatasetMetadata) -> Result<Self::Dataset, Error>;
}

/// A US state.
#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumIter,
    EnumString,
    Deserialize,
    Serialize,
)]
pub enum State {
    AL,
    AK,
    AZ,
    AR,
    CA,
    CO,
    CT,
    DE,
    FL,
    GA,
    HI,
    ID,
    IL,
    IN,
    IA,
    KS,
    KY,
    LA,
    ME,
    MD,
    MA,
    MI,
    MN,
    MS,
    MO,
    MT,
    NE,
    NV,
    NH,
    NJ,
    NM,
    NY,
    NC,
    ND,
    OH,
    OK,
    OR,
    PA,
    RI,
    SC,
    SD,
    TN,
    TX,
    UT,
    VT,
    VA,
    WA,
    WI,
    WV,
    WY,
    DC,
}

impl TryFrom<u8> for State {
    type Error = Error;

    fn try_from(id: u8) -> Result<Self, Self::Error> {
        if id == 0 {
            Err(Error::msg("state ID cannot be 0"))
        } else {
            Self::iter()
                .nth(id as usize - 1)
                .ok_or_else(|| Error::msg(format!("invalid state ID {id}")))
        }
    }
}

impl State {
    /// The numeric ID of this state.
    pub fn id(&self) -> u8 {
        Self::iter().position(|state| state == *self).unwrap() as u8 + 1
    }

    /// The full name of this state.
    pub fn name(&self) -> &'static str {
        use State::*;
        match self {
            AL => "Alabama",
            AK => "Alaska",
            AZ => "Arizona",
            AR => "Arkansas",
            CA => "California",
            CO => "Colorado",
            CT => "Connecticut",
            DE => "Delaware",
            FL => "Florida",
            GA => "Georgia",
            HI => "Hawaii",
            ID => "Idaho",
            IL => "Illinois",
            IN => "Indiana",
            IA => "Iowa",
            KS => "Kansas",
            KY => "Kentucky",
            LA => "Louisiana",
            ME => "Maine",
            MD => "Maryland",
            MA => "Massachusetts",
            MI => "Michigan",
            MN => "Minnesota",
            MS => "Mississippi",
            MO => "Missouri",
            MT => "Montana",
            NE => "Nebraska",
            NV => "Nevada",
            NH => "New Hampshire",
            NJ => "New Jersey",
            NM => "New Mexico",
            NY => "New York",
            NC => "North Carolina",
            ND => "North Dakota",
            OH => "Ohio",
            OK => "Oklahoma",
            OR => "Oregon",
            PA => "Pennsylvania",
            RI => "Rhode Island",
            SC => "South Carolina",
            SD => "South Dakota",
            TN => "Tennessee",
            TX => "Texas",
            UT => "Utah",
            VT => "Vermont",
            VA => "Virginia",
            WA => "Washington",
            WI => "West Virginia",
            WV => "Wisconsin",
            WY => "Wyoming",
            DC => "Washington, D.C.",
        }
    }
}

/// The possible statuses of a bill.
#[derive(Clone, Copy, Debug, Display, EnumString)]
pub enum Status {
    None,
    Introduced,
    Engrossed,
    Enrolled,
    Passed,
    Vetoed,
    Failed,
}

/// A US political party.
#[derive(
    Clone, Copy, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, EnumString, EnumIter,
)]
pub enum Party {
    Unknown,
    Democrat,
    Republican,
    Independent,
    Green,
    Libertarian,
    Nonpartisan,
}

impl Party {
    /// The numeric ID of this party.
    pub fn id(&self) -> u8 {
        Self::iter().position(|party| party == *self).unwrap() as u8 + 1
    }

    /// The one-letter abbreviation for this party.
    pub fn abbreviation(&self) -> &'static str {
        match self {
            Self::Unknown => "?",
            Self::Democrat => "D",
            Self::Republican => "R",
            Self::Independent => "I",
            Self::Green => "G",
            Self::Libertarian => "L",
            Self::Nonpartisan => "n/a",
        }
    }
}

/// A first, middle, and last name.
#[derive(Clone, Debug)]
pub struct Name {
    pub first: String,
    pub middle: String,
    pub last: String,
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} ", self.first)?;
        if !self.middle.is_empty() {
            write!(f, "{} ", self.middle)?;
        }
        write!(f, "{}", self.last)
    }
}

/// The full contents of a dataset.
pub trait Dataset {
    /// Information about a bill.
    type Bill: Bill;

    /// Information about a person.
    type Person: Person;

    /// Iterator over bills in this dataset.
    type Bills<'a>: Iterator<Item = Self::Bill>
    where
        Self: 'a;

    /// Iterator over people in this dataset.
    type People<'a>: Iterator<Item = Self::Person>
    where
        Self: 'a;

    /// Iterate over bills in this dataset.
    fn bills(&self) -> Self::Bills<'_>;

    /// Iterate over people in this dataset.
    fn people(&self) -> Self::People<'_>;

    /// Decompress and extract the contents of this dataset to a directory.
    fn extract(&self, dir: impl AsRef<Path>) -> Result<(), Error>;
}

/// Succinct metadata about a dataset.
pub trait DatasetMetadata {
    /// The unique identifier for this dataset in the Legiscan API.
    fn id(&self) -> String;

    /// The hash of the contents of this dataset.
    ///
    /// This can be used to quickly check if a dataset needs to be updated, by storing the latest
    /// hash for each dataset and comparing it against the hash retrieved from Legiscan.
    fn hash(&self) -> String;
}

/// Information about a bill.
pub trait Bill: Send + 'static {
    /// The unique identifier for this bill in the Legiscan API.
    fn id(&self) -> String;

    /// The hash of the contents of this bill.
    ///
    /// This can be used to quickly check if a bill needs to be updated, by storing the latest hash
    /// for each bill and comparing it against the hash retrieved from Legiscan.
    fn hash(&self) -> String;

    /// The state where this bill has been introduced.
    fn state(&self) -> State;

    /// The status of this bill.
    fn status(&self) -> Status;

    /// The short name of this bill (usually a body abbreviation and a number).
    fn name(&self) -> String;

    /// A readable title for the bill.
    ///
    /// Compared to [`name`](Self::name), [`title`](Self::title) is similarly brief, more
    /// descriptive, but less precise.
    fn title(&self) -> String;

    /// A brief summary of this bill.
    fn summary(&self) -> String;

    /// The unique [`id`](Person::id) of each sponsor of this bill.
    fn sponsors(&self) -> Vec<String>;

    /// The name of each issue this bill pertains to.
    fn issues(&self) -> Vec<String>;
}

/// Information about a person.
pub trait Person: Send + 'static {
    /// The unique identifier for this person in the Legiscan API.
    fn id(&self) -> String;

    /// The hash of the information about this person.
    ///
    /// This can be used to quickly check if a person needs to be updated, by storing the latest
    /// hash for each person and comparing it against the hash retrieved from Legiscan.
    fn hash(&self) -> String;

    /// The state where this person is a legislator.
    fn state(&self) -> State;

    /// The political party of which this person is a member.
    fn party(&self) -> Party;

    /// The person's full name.
    fn name(&self) -> Name;

    /// The name of the district this person represents.
    fn district(&self) -> String;
}
