use crate::datasource::gpkg::GpkgDatasource;
use crate::datasource::postgis::PgDatasource;
use crate::endpoints::FilterParams;
use async_trait::async_trait;
use bbox_common::ogcapi::*;
use sqlx::Result;
use std::collections::HashMap;

pub mod gpkg;
pub mod postgis;

#[derive(Clone)]
pub struct DsConnections {
    gpkg_datasources: HashMap<String, GpkgDatasource>,
    pg_datasources: HashMap<String, PgDatasource>,
}

impl DsConnections {
    pub fn new() -> Self {
        DsConnections {
            gpkg_datasources: HashMap::new(),
            pg_datasources: HashMap::new(),
        }
    }
    pub async fn add_gpkg_ds(&mut self, gpkg: &str) -> Result<()> {
        let pool = GpkgDatasource::new_pool(gpkg).await?;
        self.gpkg_datasources.insert(gpkg.to_string(), pool);
        Ok(())
    }
    pub async fn add_pg_ds(&mut self, url: &str) -> Result<()> {
        let pool = PgDatasource::new_pool(url).await?;
        self.pg_datasources.insert(url.to_string(), pool);
        Ok(())
    }
    pub fn datasource(&self, gpkg: &str) -> Option<&dyn CollectionDatasource> {
        self.gpkg_datasources
            .get(gpkg)
            .map(|ds| ds as &dyn CollectionDatasource)
    }
    /// Close all connections
    pub async fn reset_pool(&mut self) -> Result<()> {
        for (_, _pool) in &self.gpkg_datasources {
            //TODO
        }
        Ok(())
    }
}

#[async_trait]
pub trait CollectionDatasource {
    async fn collections(&self) -> Result<Vec<CoreCollection>>;
    async fn items(&self, table: &str, filter: &FilterParams) -> Result<ItemsResult>;
    async fn item(&self, table: &str, feature_id: &str) -> Result<Option<CoreFeature>>;
}

pub struct ItemsResult {
    pub features: Vec<CoreFeature>,
    pub number_matched: u64,
    pub number_returned: u64,
}
