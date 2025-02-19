pub mod mbtiles;
mod mvt;
pub mod pmtiles;
pub mod postgis;
mod postgis_queries;
#[cfg(feature = "map-server")]
pub mod wms_fcgi;
pub mod wms_http;

use crate::config::{SourceParamCfg, TileSetCfg};
use crate::service::TileService;
use crate::store::mbtiles::MbtilesStore;
use crate::store::pmtiles::PmtilesStoreReader;
use async_trait::async_trait;
use bbox_core::config::{error_exit, DatasourceCfg, NamedDatasourceCfg};
use bbox_core::endpoints::TileResponse;
use bbox_core::{Format, NamedObjectStore};
use dyn_clone::{clone_trait_object, DynClone};
use geozero::error::GeozeroError;
use martin_mbtiles::Metadata;
use tile_grid::{RegistryError, Tms, Xyz};
use tilejson::TileJSON;

#[derive(thiserror::Error, Debug)]
pub enum TileSourceError {
    #[error("tileserver.source `{0}` not found")]
    TileSourceNotFound(String),
    #[error("tileserver.source of type {0} expected")]
    TileSourceTypeError(String),
    #[error("tile not found / out of bounds")]
    TileXyzError,
    #[error(transparent)]
    RegistryError(#[from] RegistryError),
    #[error(transparent)]
    FcgiError(#[from] wms_fcgi::FcgiError),
    #[error("FCGI for suffix `{0}` not found")]
    SuffixNotFound(String),
    #[error(transparent)]
    DbError(#[from] sqlx::Error),
    #[error("Source field type detection failed")]
    TypeDetectionError,
    #[error("Integer out of range")]
    IntRangeError(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    GeozeroError(#[from] GeozeroError),
    #[error("MVT encoding error")]
    MvtEncodeError, // prost::error::EncodeError
    #[error(transparent)]
    WmsHttpError(#[from] reqwest::Error),
    #[error(transparent)]
    MbtilesError(#[from] martin_mbtiles::MbtError),
    #[error(transparent)]
    PmtilesError(#[from] ::pmtiles::error::Error),
}

#[derive(PartialEq, Clone, Debug)]
pub enum SourceType {
    Vector,
    Raster,
}

pub struct LayerInfo {
    pub name: String,
    pub geometry_type: Option<String>,
}

#[async_trait]
pub trait TileRead: DynClone + Send + Sync {
    /// Request tile from source
    async fn xyz_request(
        &self,
        service: &TileService,
        tms_id: &str,
        tile: &Xyz,
        format: &Format,
        request_params: wms_fcgi::HttpRequestParams<'_>,
    ) -> Result<TileResponse, TileSourceError>;
    /// Type information
    fn source_type(&self) -> SourceType;
    /// Default tile format
    fn default_format(&self) -> &Format {
        match self.source_type() {
            SourceType::Vector => &Format::Mvt,
            SourceType::Raster => &Format::Png, // TODO: support for "image/png; mode=8bit"
        }
    }
    /// TileJSON layer metadata (https://github.com/mapbox/tilejson-spec)
    async fn tilejson(&self, format: &Format) -> Result<TileJSON, TileSourceError>;
    /// Layer metadata
    async fn layers(&self) -> Result<Vec<LayerInfo>, TileSourceError>;
    /// MBTiles metadata.json (https://github.com/mapbox/mbtiles-spec/blob/master/1.3/spec.md)
    async fn mbtiles_metadata(
        &self,
        tileset: &TileSetCfg,
        format: &Format,
    ) -> Result<Metadata, TileSourceError> {
        // t_rex:
        // fn get_tilejson_metadata(&self, tileset: &str, grid: &Grid) -> JsonResult {
        //     let ts = self
        //         .get_tileset(tileset)
        //         .expect(&format!("Tileset '{}' not found", tileset));
        //     let ext = ts.get_extent();
        //     let center = ts.get_center();
        //     let zoom = ts.get_start_zoom();
        //     let mut meta = json!({
        //         "id": tileset,
        //         "name": tileset,
        //         "description": tileset,
        //         "attribution": ts.attribution(),
        //         "format": "pbf",
        //         "version": "2.0.0", //edition of the software, keep 2.0 for backwards compat
        //         "tilejson": "2.2.0", //edition of the tilejson standard adopted
        //         "scheme": "xyz",
        //         "bounds": [ext.minx,
        //                    ext.miny,
        //                    ext.maxx,
        //                    ext.maxy],
        //         // Minimum zoom level for which tiles are available.
        //         // Optional. Default: 0. >= 0, <= 30.
        //         "minzoom": ts.minzoom(),
        //         // Maximum zoom level for which tiles are available.
        //         // Data from tiles at the maxzoom are used when displaying the map at higher zoom levels.
        //         // Optional. Default: 30. >= 0, <= 30. (Mapbox Style default: 22)
        //         "maxzoom": ts.maxzoom(),
        //         "center": [center.0, center.1, zoom],
        //         "basename": tileset
        //     });
        //     if grid.srid != 3857 {
        //         // TODO: add full grid information according to GDAL extension
        //         // https://github.com/OSGeo/gdal/blob/release/3.4/gdal/ogr/ogrsf_frmts/mvt/ogrmvtdataset.cpp#L5497
        //         meta["srs"] = json!(format!("EPSG:{}", grid.srid));
        //     }
        //     Ok(meta)
        // }
        // fn get_mbtiles_metadata(&self, tileset: &str, grid: &Grid) -> JsonResult {
        //     let mut metadata = self.get_tilejson_metadata(tileset, grid)?;
        //     metadata["bounds"] = json!(metadata["bounds"].to_string());
        //     metadata["center"] = json!(metadata["center"].to_string());
        //     let layers = self.get_tilejson_layers(tileset)?;
        //     let vector_layers = self.get_tilejson_vector_layers(tileset)?;
        //     let metadata_vector_layers = json!({
        //         "Layer": layers,
        //         "vector_layers": vector_layers
        //     });
        //     let obj = metadata.as_object_mut().unwrap();
        //     obj.insert(
        //         "json".to_string(),
        //         json!(metadata_vector_layers.to_string()),
        //     );
        //     Ok(json!(obj))
        // }
        // let expected = r#"{
        //   "attribution": "",
        //   "basename": "osm",
        //   "bounds": "[-180.0,-90.0,180.0,90.0]",
        //   "center": "[0.0,0.0,2]",
        //   "description": "osm",
        //   "format": "pbf",
        //   "id": "osm",
        //   "json": "{\"Layer\":[{\"description\":\"\",\"fields\":{},\"id\":\"points\",\"name\":\"points\",\"properties\":{\"buffer-size\":0,\"maxzoom\":22,\"minzoom\":0}},{\"description\":\"\",\"fields\":{},\"id\":\"buildings\",\"name\":\"buildings\",\"properties\":{\"buffer-size\":10,\"maxzoom\":22,\"minzoom\":0}},{\"description\":\"\",\"fields\":{},\"id\":\"admin_0_countries\",\"name\":\"admin_0_countries\",\"properties\":{\"buffer-size\":1,\"maxzoom\":22,\"minzoom\":0}}],\"vector_layers\":[{\"description\":\"\",\"fields\":{},\"id\":\"points\",\"maxzoom\":22,\"minzoom\":0},{\"description\":\"\",\"fields\":{},\"id\":\"buildings\",\"maxzoom\":22,\"minzoom\":0},{\"description\":\"\",\"fields\":{},\"id\":\"admin_0_countries\",\"maxzoom\":22,\"minzoom\":0}]}",
        //   "maxzoom": 22,
        //   "minzoom": 0,
        //   "name": "osm",
        //   "scheme": "xyz",
        //   "srs": "EPSG:4326",
        //   "tilejson": "2.2.0",
        //   "version": "2.0.0"
        // }"#;
        Ok(Metadata {
            id: tileset.name.clone(),
            tile_info: martin_tile_utils::TileInfo {
                format: martin_tile_utils::Format::parse(format.file_suffix())
                    .unwrap_or(martin_tile_utils::Format::Mvt),
                encoding: martin_tile_utils::Encoding::Uncompressed,
            },
            tilejson: self.tilejson(format).await?,
            layer_type: None,
            json: None,
        })
    }
}

clone_trait_object!(TileRead);

/// Datasource connection pools
#[derive(Default)]
pub struct Datasources {
    pg_datasources: NamedObjectStore<postgis::Datasource>,
    // Store config for non-pooled sources
    config_sources: NamedObjectStore<DatasourceCfg>,
}

impl Datasources {
    /// Setup datasource connection pools
    pub async fn create(datasources: &Vec<NamedDatasourceCfg>) -> Self {
        // TODO: setup referenced datasources only (?)
        let mut ds_handler = Datasources::default();
        for named_ds in datasources {
            // TODO:  check duplicate names
            let ds = &named_ds.datasource;
            match ds {
                DatasourceCfg::Postgis(cfg) => ds_handler.pg_datasources.add(
                    &named_ds.name,
                    postgis::Datasource::from_config(cfg)
                        .await
                        .unwrap_or_else(error_exit),
                ),
                _ => ds_handler.config_sources.add(&named_ds.name, ds.clone()),
            }
        }
        ds_handler
    }
    /// Setup tile source instance
    pub async fn setup_tile_source(&self, cfg: &SourceParamCfg, tms: &Tms) -> Box<dyn TileRead> {
        // -- raster sources --
        // wms_fcgi::WmsFcgiSource,
        // wms_http::WmsHttpSource,
        // // GdalData(GdalSource),
        // // RasterData(GeorasterSource),
        // -- vector sources --
        // postgis::PgSource,
        // // OgrData(OgrQueries),
        // // VectorData(GeozeroSource),
        // // OsmData(OsmSource),
        // -- direct tile sources --
        // mbtiles::MbtilesSource,
        // // Pmtiles(PmtilesSource),
        // // PgTile(PgTileQueries),
        // /// dummy source for disabled features
        // Empty,
        match cfg {
            SourceParamCfg::WmsHttp(cfg) => {
                let DatasourceCfg::WmsHttp(provider) =
                    self.config_sources.get(&cfg.source).unwrap_or_else(|| {
                        error_exit(TileSourceError::TileSourceNotFound(cfg.source.clone()))
                    })
                else {
                    error_exit(TileSourceError::TileSourceTypeError(
                        "wms_proxy".to_string(),
                    ))
                };
                Box::new(wms_http::WmsHttpSource::from_config(
                    provider,
                    cfg,
                    tms.crs().as_srid(),
                ))
            }
            #[cfg(feature = "map-server")]
            SourceParamCfg::WmsFcgi(cfg) => Box::new(wms_fcgi::WmsFcgiSource::from_config(cfg)),
            #[cfg(not(feature = "map-server"))]
            SourceParamCfg::WmsFcgi(_cfg) => {
                // TODO: Emit warning
                unimplemented!()
            }
            SourceParamCfg::Postgis(pg_cfg) => {
                let ds = self
                    .pg_datasources
                    .get_or_default(pg_cfg.datasource.as_deref())
                    .unwrap_or_else(|| {
                        error_exit(TileSourceError::TileSourceNotFound(
                            pg_cfg
                                .datasource
                                .as_ref()
                                .unwrap_or(&"(default)".to_string())
                                .clone(),
                        ))
                    });
                Box::new(postgis::PgSource::create(ds, pg_cfg, tms).await)
            }
            SourceParamCfg::Mbtiles(cfg) => Box::new(
                MbtilesStore::from_config(cfg)
                    .await
                    .unwrap_or_else(error_exit),
            ),
            SourceParamCfg::Pmtiles(cfg) => Box::new(
                PmtilesStoreReader::from_config(cfg)
                    .await
                    .unwrap_or_else(error_exit),
            ),
        }
    }
}

pub fn source_config_from_cli_arg(file_or_url: &str) -> Option<SourceParamCfg> {
    MbtilesStore::config_from_cli_arg(file_or_url).map(SourceParamCfg::Mbtiles)
}

#[cfg(not(feature = "map-server"))]
pub mod wms_fcgi {
    // Replacements for bbox_map_server types
    #[derive(Default)]
    pub struct WmsMetrics;
    #[derive(Clone)]
    pub struct MapService;
    impl MapService {
        pub fn metrics(&self) -> &'static WmsMetrics {
            unimplemented!()
        }
    }
    pub type FcgiError = std::io::Error;

    pub struct HttpRequestParams<'a> {
        pub scheme: &'a str,
        pub host: &'a str,
        pub req_path: &'a str,
        pub metrics: &'a WmsMetrics,
    }
}
