use crate::config::RoutingServerCfg;
use crate::engine::Router;
use actix_web::web;
use async_trait::async_trait;
use bbox_core::cli::{NoArgs, NoCommands};
use bbox_core::config::config_error_exit;
use bbox_core::metrics::{no_metrics, NoMetrics};
use bbox_core::ogcapi::ApiLink;
use bbox_core::service::{CoreService, OgcApiService};
use clap::ArgMatches;
use log::warn;

#[derive(Clone, Default)]
pub struct RoutingService {
    pub router: Option<Router>,
}

#[async_trait]
impl OgcApiService for RoutingService {
    type CliCommands = NoCommands;
    type CliArgs = NoArgs;
    type Metrics = NoMetrics;

    async fn read_config(&mut self, _cli: &ArgMatches) {
        let Some(config) = RoutingServerCfg::from_config() else {
            warn!("No routing config available");
            self.router = None;
            return;
        };
        self.router = match config.service.len() {
            1 => {
                let service = &config.service[0];
                Some(Router::from_config(service).await.unwrap())
            }
            0 => {
                warn!("No routing config available");
                None
            }
            _ => {
                config_error_exit(figment::Error::from(
                    "Currently only one routing service supported".to_string(),
                ));
                None
            }
        };
    }
    fn conformance_classes(&self) -> Vec<String> {
        vec![
            // Core
            "http://www.opengis.net/spec/ogcapi-routes-1/1.0.0-draft.1/conf/core".to_string(),
            /*
            // Manage routes
            "http://www.opengis.net/spec/ogcapi-routes-1/1.0.0-draft.1/conf/manage-routes".to_string(),
            // Modes
            "http://www.opengis.net/spec/ogcapi-routes-1/1.0.0-draft.1/conf/mode".to_string(),
            // Intermediate waypoints
            "http://www.opengis.net/spec/ogcapi-routes-1/1.0.0-draft.1/conf/intermediate-waypoints".to_string(),
            // Height restrictions
            "http://www.opengis.net/spec/ogcapi-routes-1/1.0.0-draft.1/conf/height".to_string(),
            // Weight restrictions
            "http://www.opengis.net/spec/ogcapi-routes-1/1.0.0-draft.1/conf/weight".to_string(),
            // Obstacles
            "http://www.opengis.net/spec/ogcapi-routes-1/1.0.0-draft.1/conf/obstacles".to_string(),
            // Temporal constraints
            "http://www.opengis.net/spec/ogcapi-routes-1/1.0.0-draft.1/conf/time".to_string(),
             */
            // OpenAPI Specification
            "http://www.opengis.net/spec/ogcapi-routes-1/1.0.0-draft.1/conf/oas30".to_string(),
        ]
    }
    fn landing_page_links(&self, _api_base: &str) -> Vec<ApiLink> {
        vec![ApiLink {
            href: "/routes".to_string(),
            rel: Some("routes".to_string()),
            type_: Some("application/json".to_string()),
            title: Some("OGC API routes".to_string()),
            hreflang: None,
            length: None,
        }]
    }
    fn openapi_yaml(&self) -> Option<&str> {
        Some(include_str!("openapi.yaml"))
    }
    fn register_endpoints(&self, cfg: &mut web::ServiceConfig, core: &CoreService) {
        self.register(cfg, core)
    }
    fn metrics(&self) -> &'static Self::Metrics {
        no_metrics()
    }
}
