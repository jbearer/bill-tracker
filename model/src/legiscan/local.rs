//! A Legiscan client which reads from the local file system instead of the Legiscan API.

use super::{
    client::{Bill, People, Person, ResponseBody},
    Legiscan, State,
};
use anyhow::Error;
use async_trait::async_trait;
use copy_dir::copy_dir;
use std::marker::PhantomData;
use std::{
    fs::{self, File, ReadDir},
    path::{Path, PathBuf},
};

/// A Legiscan client which reads from the local file system instead of the Legiscan API.
pub struct LocalClient {
    root: PathBuf,
}

impl LocalClient {
    /// Open a local Legiscan dataset.
    pub fn open(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl Legiscan for LocalClient {
    type Dataset = Dataset;
    type DatasetMetadata = DatasetMetadata;

    async fn list_datasets(
        &self,
        state: Option<State>,
        year: Option<u16>,
    ) -> Result<Vec<Self::DatasetMetadata>, Error> {
        Ok(if let Some(state) = state {
            DatasetMetadata::list(year, [self.root.join(state.to_string())])
        } else {
            DatasetMetadata::list(
                year,
                self.root.read_dir()?.filter_map(|dirent| match dirent {
                    Ok(de) => Some(de.path()),
                    Err(err) => {
                        tracing::error!("unable to read directory {}: {err}", self.root.display());
                        None
                    }
                }),
            )
        })
    }

    async fn load_dataset(&self, dataset: &Self::DatasetMetadata) -> Result<Self::Dataset, Error> {
        Ok(Dataset {
            root: dataset.root.clone(),
        })
    }
}

/// Metadata about a local dataset.
pub struct DatasetMetadata {
    root: PathBuf,
    hash: String,
    id: String,
}

impl DatasetMetadata {
    fn list<I>(year: Option<u16>, states: I) -> Vec<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        states
            .into_iter()
            .flat_map(|state| {
                let state = state.as_ref();
                let years = match state.read_dir() {
                    Ok(years) => years,
                    Err(err) => {
                        tracing::error!("unable to read directory {}: {err}", state.display());
                        return vec![];
                    }
                };
                years
                    .filter_map(|dataset| {
                        let dataset = match dataset {
                            Ok(dataset) => dataset,
                            Err(err) => {
                                tracing::error!("unable to read dataset: {err}");
                                return None;
                            }
                        };

                        let root = dataset.path();
                        if let Some(year) = year {
                            if !root
                                .display()
                                .to_string()
                                .starts_with(year.to_string().as_str())
                            {
                                return None;
                            }
                        }

                        let hash = match fs::read(root.join("hash.md5")) {
                            Ok(hash) => hash,
                            Err(err) => {
                                tracing::error!(
                                    "unable to read hash file in dataset {}: {err}",
                                    root.display()
                                );
                                return None;
                            }
                        };
                        let hash = match std::str::from_utf8(&hash) {
                            Ok(hash) => hash.into(),
                            Err(err) => {
                                tracing::error!(
                                    "malformed hash file in dataset {}: {err}",
                                    root.display()
                                );
                                return None;
                            }
                        };
                        Some(Self {
                            root,
                            hash,
                            id: "TODO".into(),
                        })
                    })
                    .collect()
            })
            .collect()
    }
}

impl super::DatasetMetadata for DatasetMetadata {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn hash(&self) -> String {
        self.hash.clone()
    }
}

/// A dataset saved in the local filesystem.
pub struct Dataset {
    root: PathBuf,
}

impl super::Dataset for Dataset {
    type Bill = Bill;
    type Bills<'a> = DatasetIter<Self::Bill>;
    type Person = Person;
    type People<'a> = People<DatasetIter<Self::Person>>;

    fn bills(&self) -> Self::Bills<'_> {
        DatasetIter::new(self.root.join("bill"))
    }

    fn people(&self) -> Self::People<'_> {
        DatasetIter::new(self.root.join("people")).into()
    }

    fn extract(&self, dir: impl AsRef<Path>) -> Result<(), Error> {
        copy_dir(&self.root, dir)?;
        Ok(())
    }
}

pub struct DatasetIter<T> {
    iter: Option<ReadDir>,
    _phantom: PhantomData<fn(&T)>,
}

impl<T> DatasetIter<T> {
    fn new(path: impl AsRef<Path>) -> Self {
        Self {
            iter: match path.as_ref().read_dir() {
                Ok(reader) => Some(reader),
                Err(err) => {
                    tracing::error!("unable to read dataset {}: {err}", path.as_ref().display());
                    None
                }
            },
            _phantom: Default::default(),
        }
    }
}

impl<T: ResponseBody> Iterator for DatasetIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let iter = self.iter.as_mut()?;

        // Search for a file that we can interpret as a `T`.
        for res in iter.by_ref() {
            let dirent = match res {
                Ok(de) => de,
                Err(err) => {
                    tracing::error!("unable to read directory: {err}");
                    continue;
                }
            };
            let mut file = match File::open(dirent.path()) {
                Ok(file) => file,
                Err(err) => {
                    tracing::error!("unable to open file {}: {err}", dirent.path().display());
                    continue;
                }
            };
            let item: T::Container = match serde_json::from_reader(&mut file) {
                Ok(item) => item,
                Err(err) => {
                    tracing::error!("file {} is malformed: {err}", dirent.path().display());
                    continue;
                }
            };
            return Some(item.into());
        }

        None
    }
}
