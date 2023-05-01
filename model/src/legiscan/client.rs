//! A Legiscan client.

use super::{Legiscan, Name, Party, State, Status};
use anyhow::Error;
use async_trait::async_trait;
use base64::prelude::*;
use derive_more::Into;
use serde::{
    de::{DeserializeOwned, Deserializer, Error as _},
    Deserialize, Serialize,
};
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};
use std::marker::PhantomData;
use std::path::Path;
use zip::ZipArchive;

/// A Legiscan client.
pub struct Client {
    client: surf::Client,
    api_key: String,
}

impl Client {
    /// Connect to Legiscan.
    pub fn new(api_key: String) -> Self {
        Self {
            client: surf::Config::default()
                .set_base_url("https://api.legiscan.com/".parse().unwrap())
                .try_into()
                .unwrap(),
            api_key,
        }
    }

    fn request(&self, op: impl Into<String>) -> Request {
        Request::new(&self.client, op.into(), self.api_key.clone())
    }
}

#[async_trait]
impl Legiscan for Client {
    type Dataset = CompressedDataset;
    type DatasetMetadata = DatasetMetadata;

    async fn list_datasets(
        &self,
        state: Option<State>,
        year: Option<u16>,
    ) -> Result<Vec<Self::DatasetMetadata>, Error> {
        let mut req = self.request("getDatasetList");
        if let Some(state) = state {
            req = req.param("state", state.to_string());
        }
        if let Some(year) = year {
            req = req.param("year", year.to_string());
        }
        req.get().await
    }

    async fn load_dataset(&self, dataset: &Self::DatasetMetadata) -> Result<Self::Dataset, Error> {
        let res = self
            .request("getDataset")
            .param("id", dataset.session_id.to_string())
            .param("access_key", &dataset.access_key)
            .get::<Dataset>()
            .await?;
        res.try_into()
    }
}

/// The body of a Legiscan API response.
///
/// Successful Legiscan responses have the form
/// ```json
/// {
///     "status": "OK",
///     "data": { ... }
/// }
/// ```
/// where `"data"` depends on the endpoint.
///
/// This trait represents the nested struct with the actual response payload, which can be extracted
/// from the `"data": { ... }` container.
trait ResponseBody: Sized {
    /// The container of this payload.
    type Container: DeserializeOwned + Into<Self>;
}

/// A Legiscan response containing data of type `T`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status")]
enum Response<T> {
    #[serde(rename = "OK")]
    Ok(T),
    #[serde(rename = "ERROR")]
    Err { alert: Option<Alert> },
}

/// A message attached to an error response.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Alert {
    message: String,
}

/// Response from the `getDatasetList` endpoint.
#[derive(Clone, Debug, Deserialize, Serialize, Into)]
struct DatasetListResponse {
    datasetlist: Vec<DatasetMetadata>,
}

impl ResponseBody for Vec<DatasetMetadata> {
    type Container = DatasetListResponse;
}

/// Succinct metadata about a Legiscan dataset.
///
/// Entries in the list returned by the `getDatasetList` endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DatasetMetadata {
    state_id: u64,
    session_id: u64,
    special: u8,
    year_start: u16,
    year_end: u16,
    session_name: String,
    session_title: String,
    dataset_hash: String,
    dataset_date: String,
    dataset_size: usize,
    access_key: String,
}

impl super::DatasetMetadata for DatasetMetadata {
    fn id(&self) -> String {
        self.session_id.to_string()
    }

    fn hash(&self) -> String {
        self.dataset_hash.clone()
    }
}

/// Response from the `getDataset` endpoint.
#[derive(Clone, Debug, Deserialize, Serialize, Into)]
struct DatasetResponse {
    dataset: Dataset,
}

impl ResponseBody for Dataset {
    type Container = DatasetResponse;
}

/// Compressed dataset returned by `getDataset`.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Dataset {
    state_id: u8,
    session_id: u64,
    session_name: String,
    dataset_hash: String,
    dataset_date: String,
    dataset_size: usize,
    mime: Option<String>,
    zip: String,
}

/// A compressed Zip archive containing a Legiscan dataset.
///
/// This archive can be extracted from the response to a `getDataset` request.
pub struct CompressedDataset {
    zip: ZipArchive<Cursor<Vec<u8>>>,
}

impl TryFrom<Dataset> for CompressedDataset {
    type Error = Error;

    fn try_from(dataset: Dataset) -> Result<Self, Self::Error> {
        let bytes = BASE64_STANDARD.decode(dataset.zip)?;
        Ok(Self {
            zip: ZipArchive::new(Cursor::new(bytes))?,
        })
    }
}

impl super::Dataset for CompressedDataset {
    type Bill = Bill;
    type Bills<'a> = CompressedDatasetIter<Bill, Cursor<Vec<u8>>>;
    type Person = Person;
    type People<'a> = People<CompressedDatasetIter<Person, Cursor<Vec<u8>>>>;

    fn bills(&self) -> Self::Bills<'_> {
        CompressedDatasetIter::new(self.zip.clone(), "bill".into())
    }

    fn people(&self) -> Self::People<'_> {
        People(CompressedDatasetIter::new(
            self.zip.clone(),
            "people".into(),
        ))
    }

    fn extract(&self, dir: impl AsRef<Path>) -> Result<(), Error> {
        Ok(self.zip.clone().extract(dir)?)
    }
}

/// An iterator over a compressed dataset yield data entries of type `T`.
pub struct CompressedDatasetIter<T, R> {
    zip: ZipArchive<R>,
    entity: String,
    index: usize,
    _phantom: PhantomData<fn(&T)>,
}

impl<T, R> CompressedDatasetIter<T, R> {
    fn new(zip: ZipArchive<R>, entity: String) -> Self {
        Self {
            zip,
            entity,
            index: 0,
            _phantom: Default::default(),
        }
    }
}

impl<T: ResponseBody, R: Read + Seek> Iterator for CompressedDatasetIter<T, R> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        // Search for a file whose path matches `self.entity`. We're looking for a file of the form
        // `*/{self.entity}/*.json`.
        for i in self.index..self.zip.len() {
            let mut file = match self.zip.by_index(i) {
                Ok(file) => file,
                Err(err) => {
                    tracing::error!("unable to load file {i}: {err}");
                    continue;
                }
            };
            if !file.is_file() {
                continue;
            }
            let Some(path) = file.enclosed_name() else {
                tracing::warn!("file {i} has malformed path, skipping");
                continue;
            };
            let path = path.to_owned();
            let Some(dir) = path.parent() else {
                continue;
            };
            if !dir.ends_with(&self.entity) {
                continue;
            }

            let item: T::Container = match serde_json::from_reader(&mut file) {
                Ok(item) => item,
                Err(err) => {
                    tracing::error!("file {} is malformed: {err}", path.display());
                    continue;
                }
            };

            // Increment index to the next file, for next time.
            self.index = i + 1;
            return Some(item.into());
        }

        None
    }
}

/// Response from the `getBill` endpoint.
#[derive(Clone, Debug, Deserialize, Serialize, Into)]
struct BillResponse {
    bill: Bill,
}

impl ResponseBody for Bill {
    type Container = BillResponse;
}

/// Information about a bill.
///
/// Returned by the `getBill` endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bill {
    bill_id: u64,
    change_hash: String,
    session_id: u64,
    status: u8,
    state: State,
    bill_number: String,
    title: String,
    description: String,
    sponsors: Vec<Person>,
    subjects: Vec<Subject>,
}

impl super::Bill for Bill {
    fn id(&self) -> String {
        self.bill_id.to_string()
    }

    fn hash(&self) -> String {
        self.change_hash.clone()
    }

    fn state(&self) -> State {
        self.state
    }

    fn status(&self) -> Status {
        match self.status {
            1 => Status::Introduced,
            2 => Status::Engrossed,
            3 => Status::Enrolled,
            4 => Status::Passed,
            5 => Status::Vetoed,
            6 => Status::Failed,
            s => {
                tracing::warn!(
                    "bill {} ({} {}) has unknown status {s}",
                    self.bill_id,
                    self.state,
                    self.bill_number
                );
                Status::None
            }
        }
    }

    fn name(&self) -> String {
        self.bill_number.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn summary(&self) -> String {
        self.description.clone()
    }

    fn sponsors(&self) -> Vec<String> {
        self.sponsors
            .iter()
            .filter_map(|sponsor| {
                if sponsor.is_actually_issue_not_person().is_some() {
                    // Sometimes, Legiscan will miscategorize a subject as a person, and then bills
                    // related to that subject get the pseudo-person listed as a sponsor, rather
                    // than the subject listed in `subjects`. We need to filter these
                    // pseudo-sponsors out.
                    None
                } else {
                    Some(sponsor.people_id.to_string())
                }
            })
            .collect()
    }

    fn issues(&self) -> Vec<String> {
        self.subjects
            .iter()
            .map(|subject| subject.subject_name.clone())
            .chain(
                // Sometimes, Legiscan will miscategorize a subject as a person, and then bills
                // related to that subject get the pseudo-person listed as a sponsor, rather than
                // the subject listed in `subjects`. Check `self.sponsors` for any "people" that are
                // actually subjects.
                self.sponsors
                    .iter()
                    .filter_map(|sponsor| sponsor.is_actually_issue_not_person()),
            )
            .collect()
    }
}

/// Response from the `getPerson` endpoint.
#[derive(Clone, Debug, Deserialize, Serialize, Into)]
struct PersonResponse {
    person: Person,
}

impl ResponseBody for Person {
    type Container = PersonResponse;
}

/// Information about a person.
///
/// Returned by the `getPerson` endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Person {
    people_id: u64,
    person_hash: String,
    #[serde(deserialize_with = "deserialize_state_id")]
    state_id: State,
    party_id: String,
    name: String,
    first_name: String,
    middle_name: String,
    last_name: String,
    district: String,
}

impl super::Person for Person {
    fn id(&self) -> String {
        self.people_id.to_string()
    }

    fn hash(&self) -> String {
        self.person_hash.clone()
    }

    fn state(&self) -> State {
        self.state_id
    }

    fn party(&self) -> Party {
        match self.party_id.as_str() {
            "1" => Party::Democrat,
            "2" => Party::Republican,
            "3" => Party::Independent,
            "4" => Party::Green,
            "5" => Party::Libertarian,
            "6" => Party::Nonpartisan,
            p => {
                tracing::warn!(
                    "person {} ({} {}) has unknown party ID {p}",
                    self.people_id,
                    self.first_name,
                    self.last_name
                );
                Party::Unknown
            }
        }
    }

    fn name(&self) -> Name {
        Name {
            first: self.first_name.clone(),
            middle: self.middle_name.clone(),
            last: self.last_name.clone(),
        }
    }

    fn district(&self) -> String {
        self.district.clone()
    }
}

impl Person {
    /// Is this "person" actually an issue?
    ///
    /// Sometimes, for some states, the Legiscan API erroneously categorizes subjects (or topic) as
    /// people, so that all of the subjects related to legislation in that state will show up as
    /// people in the state dataset, and the subjects of a bill will actually show up as sponsors.
    ///
    /// This method detects this error so we can recategorize this "person" as an issue.
    ///
    /// # Returns
    ///
    /// The name of the issue if this is an issue; otherwise [`None`].
    fn is_actually_issue_not_person(&self) -> Option<String> {
        if self.first_name.is_empty() && self.last_name.is_empty() && self.district.is_empty() {
            Some(self.name.clone())
        } else {
            None
        }
    }
}

/// Iterate over people.
///
/// This iterator transforms another iterator, fixing a Legiscan bug by filtering out "people" that
/// are actually miscategorized subjects.
pub struct People<I>(I);

impl<I: Iterator<Item = Person>> Iterator for People<I> {
    type Item = Person;

    fn next(&mut self) -> Option<Person> {
        // Advance the inner iterator, skipping people that are actually subjects.
        for person in self.0.by_ref() {
            if person.is_actually_issue_not_person().is_some() {
                tracing::debug!(
                    "person {} is actually a subject, skipping",
                    person.people_id
                );
                continue;
            } else {
                return Some(person);
            }
        }

        None
    }
}

/// A subject, or topic, in the Legiscan data model.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Subject {
    subject_id: u64,
    subject_name: String,
}

struct Request {
    builder: surf::RequestBuilder,
    params: HashMap<String, String>,
}

impl Request {
    fn new(client: &surf::Client, op: String, api_key: String) -> Self {
        let mut params = HashMap::default();
        params.insert("key".into(), api_key);
        params.insert("op".into(), op);

        Self {
            builder: client.get("/"),
            params,
        }
    }

    fn param(mut self, param: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(param.into(), value.into());
        self
    }

    async fn get<T: ResponseBody>(self) -> Result<T, Error> {
        tracing::info!(?self.builder, ?self.params, "Legiscan request");
        let res: Response<T::Container> = self
            .builder
            .query(&self.params)
            .map_err(Error::msg)?
            .recv_json()
            .await
            .map_err(Error::msg)?;
        match res {
            Response::Ok(data) => Ok(data.into()),
            Response::Err { alert } => match alert {
                Some(Alert { message }) => {
                    Err(Error::msg(format!("Legiscan API error: {message}")))
                }
                None => Err(Error::msg("Legiscan API error")),
            },
        }
    }
}

fn deserialize_state_id<'a, D: Deserializer<'a>>(d: D) -> Result<State, D::Error> {
    let id = u8::deserialize(d)?;
    id.try_into().map_err(D::Error::custom)
}
