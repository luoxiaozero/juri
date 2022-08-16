use regex::Regex;

use crate::{Request, Response};
use std::{collections::HashMap, sync::Arc};

pub type Route = (String, fn(request: Request) -> Response);
#[derive(Clone)]
pub struct Router {
    pub get: Vec<Route>,
    pub post: Vec<Route>,
}

pub type MatchRoute = (String, Vec<String>, fn(request: Request) -> Response);
#[derive(Clone)]
pub struct MatchRouter {
    pub get: Vec<MatchRoute>,
    pub post: Vec<MatchRoute>,
}

pub fn handle_router(
    request: &mut Request,
    router: Arc<MatchRouter>,
) -> Option<fn(Request) -> Response> {
    let route_list;
    if request.method == "GET" {
        route_list = Some(router.get.clone());
    } else if request.method == "POST" {
        route_list = Some(router.post.clone());
    } else {
        route_list = None
    }
    if let Some(route_list) = route_list {
        for route in route_list {
            if let Some(map) = match_router_path(&route, request.path.clone()) {
                request.params_map = map;
                return Some(route.2);
            }
        }
    }
    None
}

fn match_router_path(route: &MatchRoute, path: String) -> Option<HashMap<String, String>> {
    let re = Regex::new(route.0.as_str()).unwrap();
    let caps = re.captures(&path);
    if let Some(caps) = caps {
        let mut map = HashMap::<String, String>::new();
        for (index, key) in route.1.iter().enumerate() {
            if let Some(value) = caps.get(index + 1) {
                map.insert(key.to_string(), value.as_str().to_string());
            }
        }
        return Some(map);
    }
    None
}
