use serde::Deserialize;

use crate::helpers::{Coordinate, Preference};

use super::AppState;
use crate::config::get_config;
use actix_web::web::Path;

#[derive(Deserialize)]
pub struct FspRequest {
    id: usize,
    waypoints: Vec<Coordinate>,
    alpha: Preference,
}

pub fn get_cost_tags() -> HttpResponse {
    HttpResponse::Ok().json(get_config().edge_cost_tags())
}

