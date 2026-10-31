//! Pure todo model: commands in, a list and a [`ViewState`] out. No GTK, no I/O.

use serde::{Deserialize, Serialize};

use crate::modules::ViewState;

/// Ordered most urgent first.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    #[default]
    Normal,
    Low,
}

impl Priority {
    /// Every priority, most urgent first.
    pub const ALL: [Priority; 3] = [Priority::High, Priority::Normal, Priority::Low];

    /// The name the popover shows.
    pub fn label(self) -> &'static str {
        match self {
            Priority::High => "High",
            Priority::Normal => "Normal",
            Priority::Low => "Low",
        }
    }

    /// Its stable CSS class.
    pub fn css_class(self) -> &'static str {
        match self {
            Priority::High => "zb-priority-high",
            Priority::Normal => "zb-priority-normal",
            Priority::Low => "zb-priority-low",
        }
    }

    /// The priority a click on the badge moves to: high, normal, low, then high again.
    pub fn next(self) -> Priority {
        match self {
            Priority::High => Priority::Normal,
            Priority::Normal => Priority::Low,
            Priority::Low => Priority::High,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Todo {
    pub id: u64,
    pub text: String,
    pub done: bool,
    pub priority: Priority,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TodoList {
    items: Vec<Todo>,
    next_id: u64,
}

/// Everything the popover can ask of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Add { text: String, priority: Priority },
    Toggle(u64),
    Edit { id: u64, text: String },
    SetPriority { id: u64, priority: Priority },
    Delete(u64),
}

/// Trimmed text, or `None` if nothing is left.
fn tidy(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

impl TodoList {
    pub fn apply(&mut self, command: Command) -> Option<u64> {
        match command {
            Command::Add { text, priority } => {
                let text = tidy(&text)?;
                let id = self.next_id;
                self.next_id += 1;
                self.items.push(Todo {
                    id,
                    text,
                    done: false,
                    priority,
                });
                Some(id)
            }
            Command::Toggle(id) => {
                let todo = self.items.iter_mut().find(|t| t.id == id)?;
                todo.done = !todo.done;
                Some(id)
            }
            Command::Edit { id, text } => {
                let text = tidy(&text)?;
                self.find_mut(id)?.text = text;
                Some(id)
            }
            Command::SetPriority { id, priority } => {
                self.find_mut(id)?.priority = priority;
                Some(id)
            }
            Command::Delete(id) => {
                let index = self.items.iter().position(|t| t.id == id)?;
                self.items.remove(index);
                Some(id)
            }
        }
    }

    fn find_mut(&mut self, id: u64) -> Option<&mut Todo> {
        self.items.iter_mut().find(|t| t.id == id)
    }

    /// Display order: open before done, then high → normal → low, then oldest first.
    pub fn items(&self) -> impl Iterator<Item = &Todo> {
        let mut items: Vec<&Todo> = self.items.iter().collect();
        // Stable sort keeps insertion order within equal keys.
        items.sort_by_key(|t| (t.done, t.priority));
        items.into_iter()
    }

    pub fn get(&self, id: u64) -> Option<&Todo> {
        self.items.iter().find(|t| t.id == id)
    }

    pub fn open_count(&self) -> usize {
        self.items.iter().filter(|t| !t.done).count()
    }
}

/// The bar shows the open count. Open high-priority todos add `zb-urgent` and swap the
/// icon, so urgency never relies on colour alone.
pub fn view_state(list: &TodoList) -> ViewState {
    let open = list.open_count();
    let high = list
        .items
        .iter()
        .filter(|t| !t.done && t.priority == Priority::High)
        .count();
    let tooltip = match (open, high) {
        (0, _) => "No open todos".to_string(),
        (1, 0) => "1 open todo".to_string(),
        (n, 0) => format!("{n} open todos"),
        (1, h) => format!("1 open todo, {h} high priority"),
        (n, h) => format!("{n} open todos, {h} high priority"),
    };
    let urgent = high > 0;
    ViewState {
        label: open.to_string(),
        icon: Some(if urgent { ICON_URGENT } else { ICON }.to_string()),
        classes: if urgent {
            vec!["zb-urgent"]
        } else {
            Vec::new()
        },
        tooltip: Some(tooltip),
        ..ViewState::default()
    }
}

/// The result of reading the todo file: always a usable list, plus a warning to log if
/// the file was missing or corrupt.
#[derive(Debug, PartialEq)]
pub struct Loaded {
    pub list: TodoList,
    pub warning: Option<String>,
}

/// Parse the todo file's contents (`None` if it does not exist). Never fails.
pub fn load(contents: Option<&str>) -> Loaded {
    let Some(contents) = contents else {
        return Loaded {
            list: TodoList::default(),
            warning: Some("no todo file yet, starting with an empty list".into()),
        };
    };
    match serde_json::from_str::<File>(contents) {
        Ok(file) => {
            let mut list = TodoList::default();
            for item in file.items {
                if let Some(id) = list.apply(Command::Add {
                    text: item.text,
                    priority: item.priority,
                }) && item.done
                {
                    list.apply(Command::Toggle(id));
                }
            }
            Loaded {
                list,
                warning: None,
            }
        }
        Err(e) => Loaded {
            list: TodoList::default(),
            warning: Some(format!(
                "todo file is corrupt ({e}), starting with an empty list"
            )),
        },
    }
}

impl TodoList {
    /// The file contents for this list, in insertion order.
    pub fn to_json(&self) -> String {
        let file = File {
            version: FORMAT_VERSION,
            items: self
                .items
                .iter()
                .map(|t| Item {
                    text: t.text.clone(),
                    done: t.done,
                    priority: t.priority,
                })
                .collect(),
        };
        let mut json = serde_json::to_string_pretty(&file).expect("todo list serializes");
        json.push('\n');
        json
    }
}

const FORMAT_VERSION: u32 = 1;

/// On-disk shape: `{"version": 1, "items": [{"text", "done", "priority"}]}`.
#[derive(Serialize, Deserialize)]
struct File {
    #[serde(default)]
    version: u32,
    items: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
struct Item {
    text: String,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    priority: Priority,
}

pub const ICON: &str = "zb-todo-symbolic";
pub const ICON_URGENT: &str = "zb-todo-urgent-symbolic";
