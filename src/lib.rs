use std::collections::HashMap;

use serde::{ser::SerializeMap, Serialize};

pub trait SearchQuery {
    fn get_search_query(&self) -> Query;
}

#[derive(Serialize)]
pub struct Query {
    pub query: ClauseType
}

#[derive(Serialize)]
pub struct BoolQuery {
    pub must: Vec<ClauseType>,
}

#[allow(non_camel_case_types)]
#[derive(Serialize)]
#[serde(untagged)]
pub enum ClauseType {
    String(ClauseFilter<String>),
    u32(ClauseFilter<u32>),
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClauseFilter<T: Serialize> {
    MultiMatch(MultiMatch<T>),
    Range(RangeFilter<T>),
    Terms(TermsFilter<T>),
    #[serde(rename = "bool")]
    BoolQuery(BoolQuery)
}


pub struct TermsFilter<T> where T: Serialize {
  pub field: String,
  pub terms_list: Vec<T>
}

impl<T: Serialize> Serialize for TermsFilter<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
                let mut terms = serializer.serialize_map(Some(1))?;
                terms.serialize_entry(&self.field, &self.terms_list)?;
                terms.end()
    }

}

pub struct RangeFilter<T> where T: Serialize {
    pub field: String,
    pub lte: Option<T>,
    pub gte: Option<T>
}


impl<T: Serialize> Serialize for RangeFilter<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
                let mut inner_map = HashMap::new();
                if let Some(lte) = &self.lte {
                    inner_map.insert("lte", lte);
                }

                if let Some(gte) = &self.gte {
                    inner_map.insert("gte", gte);
                }

                let mut ran = serializer.serialize_map(Some(1))?;
                ran.serialize_entry(&self.field, &inner_map)?;
                ran.end()
    }

}

#[derive(Serialize)]
pub struct MultiMatch<T: Serialize> {
    pub fields: Vec<&'static str>,
    pub query: T
}

