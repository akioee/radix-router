mod meta;
mod utils;

pub use meta::{Meta, MetaType};
use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

#[derive(Debug, Default)]
pub struct MatchedRoute {
    meta: Option<Rc<Meta>>,
    params: Option<HashMap<String, String>>,
    is_static: bool,
}

impl MatchedRoute {
    pub(crate) fn insert_params(&mut self, key: &str, value: &str) {
        if self.params.is_none() {
            self.params = Some(HashMap::from([(key.to_owned(), value.to_owned())]));
        } else {
            self.params.as_mut().unwrap().insert(key.to_owned(), value.to_owned());
        }
    }
}

#[derive(Debug)]
pub struct RouterOptions {
    pub strict_trailing_slash: bool,
}

impl Default for RouterOptions {
    fn default() -> Self {
        Self { strict_trailing_slash: false }
    }
}

#[derive(Debug)]
pub struct RouterContext {
    pub options: RouterOptions,
    pub root_route: Rc<RefCell<RouteNode>>,
    pub static_routes: HashMap<String, Rc<RefCell<RouteNode>>>,
}

impl RouterContext {
    pub fn new() -> Self {
        Self {
            options: RouterOptions::default(),
            root_route: Rc::new(RefCell::new(RouteNode::default())),
            static_routes: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteType {
    Normal,
    Wildcard,
    Placeholder,
}

#[derive(Debug)]
pub struct RouteNode {
    pub r#type: RouteType,
    pub max_depth: u8,
    pub parent: Option<Rc<RefCell<RouteNode>>>,
    pub children: HashMap<String, Rc<RefCell<RouteNode>>>,
    pub meta: Option<Rc<Meta>>,
    pub param_name: Option<String>,
    pub wildcard_child: Option<Rc<RefCell<RouteNode>>>,
    pub placeholder_children: Vec<Rc<RefCell<RouteNode>>>,
}

impl RouteNode {
    pub fn set_type(&mut self, r#type: RouteType) -> &mut Self {
        self.r#type = r#type;
        self
    }

    pub fn set_parent(&mut self, parent_node: Rc<RefCell<RouteNode>>) -> &mut Self {
        self.parent = Some(parent_node);
        self
    }
}

impl Default for RouteNode {
    fn default() -> Self {
        Self {
            r#type: RouteType::Normal,
            max_depth: 0,
            parent: None,
            children: HashMap::new(),
            meta: None,
            param_name: None,
            wildcard_child: None,
            placeholder_children: vec![],
        }
    }
}

#[derive(Debug)]
pub struct Router {
    pub context: RouterContext,
}

unsafe impl Send for Router {}

impl Router {
    pub fn new() -> Self {
        Self { context: RouterContext::new() }
    }

    pub fn insert(&mut self, path: &str, meta: Meta) {
        let mut is_static = true;
        let mut current_node = self.context.root_route.clone();
        let mut unnamed_placeholder_ctr = 0u8;
        let mut matched_nodes = vec![current_node.clone()];
        let path = normalize_trailing_slash(path, self.context.options.strict_trailing_slash);
        let sections = path.split('/').collect::<Vec<_>>();

        for section in sections {
            // path: a/b/c, b is child_node of the a
            let mut _child_node = None;

            if current_node.borrow().children.contains_key(section) {
                _child_node = Some(current_node.borrow().children.get(section).unwrap().clone())
            } else {
                let route_type = get_node_type(section);
                // Create new node to represent the next part of the path
                let mut temp_node = RouteNode::default();

                temp_node.set_type(route_type).set_parent(current_node.clone());
                _child_node = Some(Rc::new(RefCell::new(temp_node)));
                current_node.borrow_mut().children.insert(section.to_owned(), _child_node.as_ref().unwrap().clone());

                if matches!(route_type, RouteType::Placeholder) {
                    _child_node.as_ref().unwrap().borrow_mut().param_name = Some(ternary!(
                        section.eq("*"),
                        format!("_{}", unnamed_placeholder_ctr),
                        section
                            .trim_matches(':') // :apple -> apple
                            .to_owned()
                    ));
                    unnamed_placeholder_ctr = unnamed_placeholder_ctr.checked_add(1).expect("Maximum placeholder path depth reached");

                    // Collect placeholder child route node
                    current_node.borrow_mut().placeholder_children.push(_child_node.as_ref().unwrap().clone());
                    is_static = false;
                } else if matches!(route_type, RouteType::Wildcard) {
                    let param_name = section.chars().skip(3).collect::<String>();

                    _child_node.as_ref().unwrap().borrow_mut().param_name = Some(ternary!(param_name.is_empty(), "_".to_owned(), param_name));
                    current_node.borrow_mut().wildcard_child = Some(_child_node.as_ref().unwrap().clone());
                    is_static = false;
                }

                current_node = _child_node.as_ref().unwrap().clone();
                matched_nodes.push(_child_node.unwrap());
            }
        }

        for (depth, node) in matched_nodes.iter().enumerate() {
            let node_max_depth = node.borrow().max_depth;

            node.borrow_mut().max_depth = std::cmp::max((matched_nodes.len() - depth) as u8, node_max_depth);
        }

        current_node.borrow_mut().meta = Some(Rc::new(meta));

        if is_static {
            self.context.static_routes.insert(path.to_owned(), current_node);
        }
    }

    pub fn lookup(&self, path: &str) -> Option<MatchedRoute> {
        let mut matched_route = MatchedRoute::default();

        if let Some(static_path_route) = self.context.static_routes.get(path) {
            matched_route.is_static = true;

            if let Some(m) = static_path_route.borrow().meta.as_ref() {
                matched_route.meta = Some(m.clone())
            }

            return Some(matched_route);
        }

        let sections = path.split('/').collect::<Vec<_>>();
        let mut wildcard_node = None;
        let mut wildcard_parma = None;
        let mut current_node = Some(self.context.root_route.clone());

        for (idx, &section) in sections.iter().enumerate() {
            if current_node.as_ref().unwrap().borrow().wildcard_child.is_some() {
                wildcard_node = current_node.as_ref().unwrap().borrow().wildcard_child.clone();
                wildcard_parma = Some((&sections[idx..]).join("/"));
            }

            let next_node_option = {
                let current_node_ref = current_node.as_ref().unwrap().borrow();

                current_node_ref.children.get(section).cloned()
            };

            if let Some(next_node) = next_node_option {
                current_node = Some(next_node);
            } else {
                let mut temp_node = None;

                if current_node.as_ref().unwrap().borrow().placeholder_children.len() > 1 {
                    let remaining = sections.len() - idx;

                    if let Some(z) = current_node
                        .unwrap()
                        .borrow()
                        .placeholder_children
                        .iter()
                        .find(|&c| c.borrow().max_depth == remaining as u8)
                    {
                        temp_node = Some(z.clone());
                    }
                } else {
                    if let Some(z) = current_node.unwrap().borrow().placeholder_children.get(0) {
                        temp_node = Some(z.clone());
                    }
                }

                current_node = temp_node;

                if current_node.is_none() {
                    break;
                }

                if let Some(pn) = current_node.as_ref().unwrap().borrow().param_name.as_ref() {
                    matched_route.insert_params(pn, section);
                }
            }
        }

        if (current_node.is_none() || current_node.as_ref().unwrap().borrow().meta.is_none()) && wildcard_node.is_some() {
            current_node = wildcard_node;

            matched_route.insert_params(
                if let Some(pn) = current_node.as_ref().unwrap().borrow().param_name.as_ref() {
                    pn
                } else {
                    "_"
                },
                &wildcard_parma.unwrap(),
            );
        }

        if current_node.is_none() {
            return None;
        }

        if let Some(m) = current_node.as_ref().unwrap().borrow().meta.as_ref() {
            matched_route.meta = Some(m.clone());
        }

        Some(matched_route)
    }

    pub fn remove(&mut self, path: &str) -> bool {
        let mut success = false;
        let sections = path.split('/').collect::<Vec<_>>();
        let mut current_node = Some(self.context.root_route.clone());

        for &section in &sections {
            if let Some(node) = current_node.as_ref().and_then(|n| Some(n.clone())) {
                if let Some(child_node) = node.borrow().children.get(section) {
                    current_node = Some(child_node.clone());
                } else {
                    return success;
                }
            }
        }

        let current_node = current_node.as_ref().unwrap().clone();

        if current_node.borrow().meta.is_some() {
            let last_section = sections.last();

            current_node.borrow_mut().meta = None;

            if current_node.borrow().children.len() == 0 && current_node.borrow().parent.is_some() {
                current_node
                    .borrow_mut()
                    .parent
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .children
                    .remove(*last_section.unwrap_or(&""));

                current_node.borrow_mut().parent.as_ref().unwrap().borrow_mut().wildcard_child = None;
                current_node.borrow_mut().parent.as_ref().unwrap().borrow_mut().placeholder_children.clear();
            }

            success = true
        }

        self.context.static_routes.remove(path);

        success
    }
}

pub fn normalize_trailing_slash(path: &str, strict_trailing_slash: bool) -> &str {
    ternary!(
        strict_trailing_slash,
        path,
        ternary!(path.eq("/"), path, ternary!(path.is_empty(), "/", path.trim_end_matches("/")))
    )
}

pub fn get_node_type(section: &str) -> RouteType {
    if section.starts_with("**") {
        return RouteType::Wildcard;
    }

    if section.starts_with(':') || section.eq("*") {
        return RouteType::Placeholder;
    }

    RouteType::Normal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_trailing_slash() {
        assert_eq!(normalize_trailing_slash("", true), "");
        assert_eq!(normalize_trailing_slash("", false), "/");
        assert_eq!(normalize_trailing_slash("/", true), "/");
        assert_eq!(normalize_trailing_slash("/", false), "/");
        assert_eq!(normalize_trailing_slash("/example/", true), "/example/");
        assert_eq!(normalize_trailing_slash("/example/", false), "/example");
    }

    #[test]
    fn test_get_node_type() {
        assert_eq!(get_node_type("example"), RouteType::Normal);
        assert_eq!(get_node_type(""), RouteType::Normal);
        assert_eq!(get_node_type(":name"), RouteType::Placeholder);
        assert_eq!(get_node_type("**"), RouteType::Wildcard);
        assert_eq!(get_node_type("**:name"), RouteType::Wildcard);
    }

    #[test]
    fn insert_static_route() {
        let mut router = Router::new();

        router.insert("/a/b", Meta::default());
        router.insert("/e/f", Meta::default());
        router.insert("/a/b/c", Meta::default());

        assert_eq!(router.context.static_routes.len(), 3)
    }

    #[test]
    fn lookup_route() {
        let mut router = Router::new();

        router.insert("/a/b", Meta::default());

        assert!(router.lookup("/a/b").is_some());
        assert!(router.lookup("/a/b/c").is_none());
        assert!(router.lookup("/a/c").is_none());
    }

    #[test]
    fn remove_route() {
        let mut router = Router::new();

        router.insert("/a/b", Meta::default());
        router.insert("/c/d", Meta::default());

        assert!(router.remove("/a/b"));
        assert!(router.lookup("/a/b").is_none());
        assert!(router.lookup("/c/d").is_some());
    }

    #[test]
    fn insert_dyn_route() {
        let mut router = Router::new();

        router.insert("/a/:name/:age", Meta::default());

        let res = router.lookup("/a/foo_route/18");

        assert!(res.as_ref().is_some());
        assert!(!res.as_ref().unwrap().is_static);
        assert!(res.as_ref().unwrap().params.is_some());
        assert_eq!(res.as_ref().unwrap().params.as_ref().unwrap().get("name"), Some(&"foo_route".to_owned()));
        assert_eq!(res.as_ref().unwrap().params.as_ref().unwrap().get("age"), Some(&"18".to_owned()));
    }

    #[test]
    fn insert_dyn_wildcard_route() {
        let mut router = Router::new();

        router.insert("/a/**/b/c", Meta::default());
        router.insert("x/**:name/y/z", Meta::default());

        let res_1 = router.lookup("/a/chicken/18/b/c");
        let res_2 = router.lookup("x/cat/dog/18/y/z");

        assert!(res_1.as_ref().is_some());
        assert!(res_2.as_ref().is_some());
        assert!(res_1.as_ref().unwrap().params.is_some());
        assert!(res_2.as_ref().unwrap().params.is_some());
        assert_eq!(res_1.as_ref().unwrap().params.as_ref().unwrap().get("_"), Some(&"chicken/18/b/c".to_owned()));
        assert_eq!(res_2.as_ref().unwrap().params.as_ref().unwrap().get("name"), Some(&"cat/dog/18/y/z".to_owned()));
    }
}
