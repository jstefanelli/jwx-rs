use std::collections::HashMap;
use std::fmt::Display;
use crate::behaviours::behaviour::Behaviour;
use crate::behaviours::behaviour_router::RoutePartType::PARAMETER;
use crate::http::http_message::HttpVersion;
use crate::http::http_request::HttpRequest;
use crate::http::http_response::HttpResponse;

#[derive(PartialEq, Debug)]
pub enum RoutePartType {
    PLAIN,
    PARAMETER,
    IGNORE
}

pub struct RoutePart {
    name: String,
    part_type: RoutePartType
}

pub struct Route {
    parts: Vec<RoutePart>
}

impl Route {
    pub fn parse(route: &str) -> Route {
        let mut sections: Vec<String> = Vec::new();

        let mut idx = route.find('/');

        let mut rt = match idx {
            Some(0) => {
                let r = route[1..].to_string();
                idx = r.find('/');
                r
            },
            _ => route.to_string()
        };

        while let Some(id) = idx {
            let part = rt[0..id].to_string();
            sections.push(part);
            rt = rt[(id + 1)..].to_string();
            idx = rt.find('/');
        }

        if rt.len() != 0 {
            sections.push(rt.to_string());
        }

        let mut parts: Vec<RoutePart> = Vec::new();

        for s in sections {
            let mut tp = RoutePartType::PLAIN;
            let mut sx = s.clone();
            if s.len() > 0 && s.chars().nth(0) == Some('{') && s.chars().nth(s.len() - 1) == Some('}') {
                tp = if s.len() > 2 { RoutePartType::PARAMETER } else { RoutePartType::IGNORE };
                sx = s[1..s.len() - 1].to_string();
            }

            parts.push(RoutePart {
                name: sx,
                part_type: tp
            });
        }

        Route {
            parts
        }
    }
}

impl Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();

        for part in &self.parts {
            s.push_str("/");
            match part.part_type {
                RoutePartType::PLAIN => {
                    s.push_str(&part.name);
                },
                RoutePartType::PARAMETER => {
                    s.push_str("{");
                    s.push_str(&part.name);
                    s.push_str("}");
                },
                RoutePartType::IGNORE => {
                    s.push_str("{}");
                }
            }
        }

        write!(f, "{}", s)
    }
}

fn not_found(ver: HttpVersion) -> HttpResponse {
    let content = "404: Not Found".as_bytes();
    HttpResponse::new(
        404,
        HashMap::from([("Content-Length".to_string(), format!("{}", content.len()))]),
        content.to_vec(),
        ver
    )
}

struct RouteTreeLeaf {
    leaves: HashMap<String, RouteTreeLeaf>,
    behaviour: Option<Box< dyn Behaviour>>,
    route: Route
}

pub struct BehaviourRouter {
    tree: RouteTreeLeaf
}

impl BehaviourRouter {
    pub fn new(mut behaviours: HashMap<String, Box<dyn Behaviour>>) -> BehaviourRouter {
        let mut tree = RouteTreeLeaf {
            leaves: HashMap::new(),
            behaviour: None,
            route: Route::parse("")
        };

        let keys = behaviours.keys().cloned().collect::<Vec<String>>();

        for k in keys {
            let route = Route::parse(&k);

            let mut current = &mut tree;

            let mut level: usize = 0;
            while level < route.parts.len() {
                let identifier = &route.parts[level];

                let section_name = if identifier.part_type == RoutePartType::PLAIN {
                    identifier.name.clone()
                } else {
                    "/".to_string()
                };

                let leaves = &mut current.leaves;

                current = leaves.entry(section_name).or_insert(RouteTreeLeaf {
                    leaves: HashMap::new(),
                    behaviour: None,
                    route: Route::parse("")
                });

                level += 1;
            }

            if let Some(v) = behaviours.remove(&k) {
                current.behaviour = Some(v)
            }
            current.route = route;
        }

        BehaviourRouter {
            tree
        }
    }

    fn parse_request_level<'a>(&'a self, current: &'a RouteTreeLeaf, sections: &Vec<String>, current_section: usize) -> Vec<(&'a Route, &'a Box<dyn Behaviour>)> {
        let mut res = Vec::<(&Route, & Box<dyn Behaviour>)>::new();

        if let Some(b) = &current.behaviour {
            res.push((&current.route, b));
        }

        if current_section < sections.len() {
            let section = &sections[current_section];

            if let Some(next) = current.leaves.get(section) {
                let mut val = self.parse_request_level(next, sections, current_section + 1);
                res.append(&mut val);
            }

            if let Some(next) = current.leaves.get("*") {
                let mut val = self.parse_request_level(next, sections, current_section + 1);
                res.append(&mut val);
            }
        }

        res
    }

    fn get_uri_parts(uri: &str) -> Vec<String> {
        let mut parts = Vec::new();

        let mut i: usize = 0;
        while i < uri.len() {
            match uri[(i + 1)..].find('/') {
                None => {
                    if uri.len() - 1 > i {
                        parts.push(uri[(i + 1)..].to_string());
                    }
                    break;
                },
                Some(idx) => {
                    parts.push(uri[(i + 1)..(i + 1 + idx)].to_string());
                    i += 1;
                }
            }
        }

        parts
    }

    pub fn run(&self, req: &HttpRequest) -> HttpResponse {

        let uri = &req.url.uri;
        let parts = BehaviourRouter::get_uri_parts(uri);

        let result = self.get(uri);

        let (route, behaviour) = match result {
            Some((r, b)) => (r, b),
            None => {
                return not_found(req.version.clone());
            }
        };

        let mut parameters: HashMap<String, String> = HashMap::new();

        for i in 0..parts.len() {
            if i >= route.parts.len() {
                //TODO: Print warning for more URI segments that Route parts
                break;
            }

            let p = &route.parts[i];
            if p.part_type == PARAMETER {
                parameters.insert(p.name.clone(), parts[i].clone());
            }
        }

        match behaviour.run(req, parameters) {
            Ok(resp) => resp,
            Err(e) => {
                let content = format!("500: Internal server error: {:?}", e);
                let content_bytes = content.as_bytes();
                HttpResponse::new(500, HashMap::from([("Content-Length".to_string(), content_bytes.len().to_string())]), content_bytes.to_vec(), req.version.clone())
            }
        }
    }

    pub fn get(&self, uri: &str) -> Option<(&Route, &Box<dyn Behaviour>)> {
        let parts = BehaviourRouter::get_uri_parts(uri);

        let result = self.parse_request_level(&self.tree, &parts, 0);

        let mut selected_behaviour: Option<& Box<dyn Behaviour>> = None;
        let mut selected_route: Option<&Route> = None;

        for (r, b) in result {
            match selected_route {
                Some(selected) => {
                    if selected.parts.len() < r.parts.len() {
                        selected_route = Some(r);
                        selected_behaviour = Some(b);
                    }
                },
                None => {
                    selected_route = Some(r);
                    selected_behaviour = Some(b);
                }
            }
        }

        match (selected_route, selected_behaviour) {
            (Some(r), Some(b)) => {
                Some((r, b))
            },
            _ => None
        }
    }
}