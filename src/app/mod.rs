
use std::collections::VecDeque;

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

extern crate derive_more;
use derive_more::Display;

#[derive(Clone, Debug, Display, PartialEq)]
pub enum Key {
    #[display(fmt = "{}", _0)]
    MapKey(String),
    #[display(fmt = "{}", _0)]
    ArrayIndex(usize),
}
pub type Value = serde_json::Value; // TODO: remove this?
pub type Node = serde_json::Value;

impl From<&String> for Key {
    fn from(s: &String) -> Key {
        Key::MapKey(s.to_string())
    }
}

impl From<usize> for Key {
    fn from(i: usize) -> Key {
        Key::ArrayIndex(i)
    }
}

fn describe_node_key(node: &Node, key: &Key) -> String {
    match key {
        Key::MapKey(s) => s.to_string(),
        Key::ArrayIndex(i) => match node[i].as_str() {
            Some(s) => s.to_owned(),
            _ => "".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptionItem<'a> {
    key: Key,
    child: &'a Value,
}

impl std::fmt::Display for OptionItem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.key {
            Key::MapKey(s) => write!(f, "{}", s),
            Key::ArrayIndex(_) => write!(f, "{}", self.child.as_str().unwrap_or("")), // TODO: handle the absence of a string here a bit better
        }
    }
}

// TODO:
// impl AppState for JsonAppState
trait AppState {
    fn get_filter(&self) -> &String;
}

// Implementation of ToOwned?
#[derive(Clone, Debug)]
struct SelectionMeta<'a> {
    /// The text supplied to select this value
    filter: String,
    /// Parent node
    parent: &'a Node,
    /// List of choices
    choices: Option<ReadOnlyZipper<Key>>,
}

#[derive(Debug, Clone)]
struct ReadOnlyZipper<T> {
    /// Jokers to the left
    left: VecDeque<T>,
    /// Selected item. Can be one of many filtered items.
    selected: T,
    /// Clowns to the right
    right: VecDeque<T>,
}

impl<T> ReadOnlyZipper<T> {
    pub fn new(left: VecDeque<T>, selected: T, right: VecDeque<T>) -> ReadOnlyZipper<T> {
        ReadOnlyZipper {
            left,
            selected,
            right,
        }
    }

    pub fn right(&self) -> &VecDeque<T> {
        &self.right
    }

    pub fn left(&self) -> &VecDeque<T> {
        &self.left
    }

    pub fn selected(&self) -> &T {
        &self.selected
    }

    pub fn select_prev(&mut self) {
        if let Some(from_left) = self.left.pop_back() {
            let previous_selection = std::mem::replace(&mut self.selected, from_left);
            self.right.push_front(previous_selection);
        }
    }

    pub fn select_next(&mut self) {
        if let Some(from_right) = self.right.pop_front() {
            let previous_selection = std::mem::replace(&mut self.selected, from_right);
            self.left.push_back(previous_selection);
        }
    }
}

impl<'a> SelectionMeta<'_> {
    pub fn new(parent: &'a Node) -> SelectionMeta<'a> {
        let filter = String::new();
        SelectionMeta {
            parent,
            choices: SelectionMeta::build_choices(parent, &filter),
            filter,
        }
    }

    // TODO: think about where this is used- does it make sense to live where it does? etc.
    pub fn describe_selected_key(&self) -> String {
        match &self.choices {
            Some(cs) => describe_node_key(self.parent, &cs.selected),
            None => String::new(),
        }
    }

    fn node_type(&self) -> NodeType {
        match self.parent {
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => NodeType::Branch,
            _ => NodeType::Leaf,
        }
    }

    fn build_choices(node: &'a Node, filter: &String) -> Option<ReadOnlyZipper<Key>> {
        let matcher = SkimMatcherV2::default();
        let mut right = match node {
            Node::Object(o) => o.iter()
                .filter(move |(k, _)| matcher.fuzzy_match(k, &filter).is_some())
                .map(|(k, _)| Key::MapKey(k.to_string()))
                .collect(),
            Node::Array(a) => a.into_iter().enumerate().filter(move |(_, v)| {
                match v {
                    serde_json::Value::String(s) => matcher.fuzzy_match(&s.to_string(), &filter).is_some(),
                    // TODO: we should be able to display other types here too, in particular
                    // numbers etc. Perhaps we just leave it up to the user to provide the input
                    // they want? Could possibly provide some display control.. Or just.. not..
                    // Arguably that's jq's job.
                    _ => false,
                }
            }).map(|(i, _)| Key::ArrayIndex(i)).collect(),
            _ => VecDeque::new(),
        };
        let selected = right.pop_front();
        selected.and_then(move |s| Some(ReadOnlyZipper::new(VecDeque::new(), s, right)))
    }

    pub fn choices(&'a self) -> Option<(&'a VecDeque<Key>, &'a Key, &'a VecDeque<Key>)> {
        self.choices.as_ref().and_then(|cs| Some((
            cs.left(),
            cs.selected(),
            cs.right(),
        )))
    }

    pub fn selected(&self) -> Option<Key> {
        self.choices.as_ref().and_then(|cs| Some(cs.selected().clone()))
    }

    pub fn set_filter(&mut self, s: String) {
        self.filter = s;
        self.choices = SelectionMeta::build_choices(self.parent, &self.filter)
    }

    pub fn select_prev(&mut self) {
        if let Some(choices) = self.choices.as_mut() {
            choices.select_prev()
        }
    }

    pub fn select_next(&mut self) {
        if let Some(choices) = self.choices.as_mut() {
            choices.select_next()
        }
    }
}

pub struct History<'a> {
    inner: Vec<SelectionMeta<'a>>,
}

impl<'a> std::fmt::Display for History<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.inner.len() > 0 {
            write!(f, "{}", self.inner[0].describe_selected_key())?;
            for i in self.inner.iter().skip(1) {
                write!(f, " {}", i.describe_selected_key())?
            }
        }
        std::fmt::Result::Ok(())
    }
}

impl<'a> History<'a> {
    pub fn new() -> History<'a> {
        History {
            inner: Vec::new(),
        }
    }

    fn push(&mut self, m: SelectionMeta<'a>) -> Result<(), ()> {
        match m.selected() {
            Some(_) => {
                self.inner.push(m);
                Ok(())
            }
            None => {
                Err(())
            }
        }
    }

    fn pop(&mut self) -> Option<SelectionMeta<'a>> {
        self.inner.pop()
    }

    pub fn iter(&self) -> impl Iterator<Item = String> + '_ {
        self.inner.iter().map(|o| (o.describe_selected_key()))
    }
}

#[derive(Debug, PartialEq)]
pub enum NodeType {
    /// A node corresponding to a JSON object or array
    Branch,
    /// A leaf corresponding to a JSON string
    Leaf,
}

// TODO: this whole implementation could probably be replaced by a much nicer/simpler tree
// traversal. Build a tree on top of the serde_json tree to allow easy referral to parent/child
// objects, then traverse.. Maybe it's even possible to do this during deserialization (and
// subsequently reject invalid input). Some of this implementation could be useful for a more
// general graph traversal though; as could 
/// AppState holds the state of the application
pub struct JsonAppState<'a> {
    /// Previously selected nodes
    history: History<'a>,
    /// Node currently being filtered
    current: SelectionMeta<'a>,
}

impl<'a> JsonAppState<'_> {
    pub fn new(root: &'a Node) -> JsonAppState<'a> {
        JsonAppState {
            history: History::new(),
            current: SelectionMeta::new(root),
        }
    }

    pub fn get_filter(&self) -> &String {
        &self.current.filter
    }

    pub fn set_filter(&mut self, f: String) {
        self.current.set_filter(f);
    }

    // TEST:
    pub fn select_next(&mut self) {
        self.current.select_next();
    }

    pub fn select_prev(&mut self) {
        self.current.select_prev();
    }

    pub fn choices(&self) -> Option<(Vec<String>, String, Vec<String>)> {
        self.current.choices().and_then(|(left, selected, right)| Some((
            left.iter().map(|k| describe_node_key(self.current.parent, k)).collect(),
            describe_node_key(self.current.parent, selected),
            right.iter().map(|k| describe_node_key(self.current.parent, k)).collect(),
        )))
    }

    pub fn push_selection(&'a mut self) -> NodeType {
        if let Some(key) = self.current.selected() {
            let child = match key {
                Key::ArrayIndex(i) => &self.current.parent[i],
                Key::MapKey(s) => &self.current.parent[s],
            };
            self.history.push(std::mem::replace(&mut self.current, SelectionMeta::new(child)));
        }
        self.current.node_type()
    }

    pub fn pop_selection(&mut self) {
        if let Some(mut prev) = self.history.pop() {
            std::mem::swap(&mut prev, &mut self.current);
        }
    }

    pub fn get_history(&self) -> &History {
        &self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let data = serde_json::json!(
            {
                "systemctl": {
                    "--system": {
                        "restart": [
                            "signal",
                            "firefox",
                            "gmail"
                        ]
                    },
                    "--user": {
                        "restart": [
                            "signal",
                            "firefox",
                            "gmail"
                        ]
                    }
                }
            }
        );
        let o = data.as_object().unwrap();
        let mut app = JsonAppState::new(o);

        app.set_filter("sys".to_string());
        app.push_selection();
        assert_eq!(app.current, data["systemctl"].as_object().unwrap());

        app.set_filter("sys".to_string());
        app.push_selection();
        assert_eq!(app.current, data["systemctl"]["--system"].as_object().unwrap());

        println!("app current before {:?}", app.current);
        app.set_filter("res".to_string());
        app.push_selection();
        println!("app current after {:?}", app.current);
        assert_eq!(app.current, data["systemctl"]["--system"]["restart"].as_object().unwrap());
    }
}
