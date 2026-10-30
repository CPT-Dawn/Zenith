use pretty_assertions::assert_eq;
use zenith_bar::modules::todo::model::{self, Command, Priority, TodoList};

fn add(list: &mut TodoList, text: &str, priority: Priority) -> u64 {
    list.apply(Command::Add {
        text: text.into(),
        priority,
    })
    .expect("added")
}

#[test]
fn adding_todos_counts_them_on_the_bar() {
    let mut list = TodoList::default();
    add(&mut list, "water the plants", Priority::Normal);
    add(&mut list, "file taxes", Priority::Normal);

    let state = model::view_state(&list);

    assert_eq!(state.label, "2");
    assert!(state.visible);
}

#[test]
fn completing_a_todo_takes_it_off_the_count_and_toggling_again_reopens_it() {
    let mut list = TodoList::default();
    let plants = add(&mut list, "water the plants", Priority::Normal);
    add(&mut list, "file taxes", Priority::Normal);

    list.apply(Command::Toggle(plants));
    assert_eq!(model::view_state(&list).label, "1");
    assert!(list.get(plants).expect("still listed").done);

    list.apply(Command::Toggle(plants));
    assert_eq!(model::view_state(&list).label, "2");
}

#[test]
fn editing_changes_the_text_and_priority_in_place() {
    let mut list = TodoList::default();
    let taxes = add(&mut list, "file taxs", Priority::Normal);

    list.apply(Command::Edit {
        id: taxes,
        text: "  file taxes ".into(),
    });
    list.apply(Command::SetPriority {
        id: taxes,
        priority: Priority::High,
    });

    let todo = list.get(taxes).expect("still listed");
    assert_eq!(todo.text, "file taxes");
    assert_eq!(todo.priority, Priority::High);
}

#[test]
fn deleting_removes_the_todo_for_good() {
    let mut list = TodoList::default();
    let plants = add(&mut list, "water the plants", Priority::Normal);
    add(&mut list, "file taxes", Priority::Normal);

    assert_eq!(list.apply(Command::Delete(plants)), Some(plants));

    assert_eq!(list.get(plants), None);
    assert_eq!(model::view_state(&list).label, "1");
    assert_eq!(list.apply(Command::Delete(plants)), None);
}

fn texts(list: &TodoList) -> Vec<&str> {
    list.items().map(|t| t.text.as_str()).collect()
}

#[test]
fn open_todos_come_first_by_priority_then_in_the_order_they_were_added() {
    let mut list = TodoList::default();
    add(&mut list, "low one", Priority::Low);
    add(&mut list, "normal one", Priority::Normal);
    let done = add(&mut list, "high but done", Priority::High);
    add(&mut list, "high one", Priority::High);
    add(&mut list, "normal two", Priority::Normal);
    add(&mut list, "high two", Priority::High);
    list.apply(Command::Toggle(done));

    assert_eq!(
        texts(&list),
        [
            "high one",
            "high two",
            "normal one",
            "normal two",
            "low one",
            "high but done",
        ]
    );
}

#[test]
fn raising_a_priority_moves_the_todo_up() {
    let mut list = TodoList::default();
    add(&mut list, "first", Priority::Normal);
    let second = add(&mut list, "second", Priority::Normal);

    list.apply(Command::SetPriority {
        id: second,
        priority: Priority::High,
    });

    assert_eq!(texts(&list), ["second", "first"]);
}

#[test]
fn an_open_high_priority_todo_makes_the_bar_stand_out_beyond_colour() {
    let mut list = TodoList::default();
    add(&mut list, "file taxes", Priority::Normal);
    let calm = model::view_state(&list);
    assert_eq!(calm.classes, Vec::<&str>::new());
    assert_eq!(calm.icon.as_deref(), Some("zb-todo-symbolic"));
    assert_eq!(calm.tooltip.as_deref(), Some("1 open todo"));

    let fire = add(&mut list, "put out the fire", Priority::High);
    let urgent = model::view_state(&list);
    assert_eq!(urgent.classes, ["zb-urgent"]);
    assert_eq!(urgent.icon.as_deref(), Some("zb-todo-urgent-symbolic"));
    assert_eq!(
        urgent.tooltip.as_deref(),
        Some("2 open todos, 1 high priority")
    );

    list.apply(Command::Toggle(fire));
    assert_eq!(model::view_state(&list), calm);
}

#[test]
fn an_empty_list_still_shows_so_there_is_somewhere_to_click() {
    let state = model::view_state(&TodoList::default());

    assert_eq!(state.label, "0");
    assert!(state.visible);
    assert_eq!(state.tooltip.as_deref(), Some("No open todos"));
    assert_eq!(state.next_wake, None);
}

#[test]
fn a_saved_list_reads_back_the_same() {
    let mut list = TodoList::default();
    add(&mut list, "low one", Priority::Low);
    let done = add(&mut list, "done one", Priority::High);
    add(&mut list, "high one", Priority::High);
    list.apply(Command::Toggle(done));

    let loaded = model::load(Some(&list.to_json()));

    assert_eq!(loaded.warning, None);
    let summary = |l: &TodoList| {
        l.items()
            .map(|t| (t.text.clone(), t.done, t.priority))
            .collect::<Vec<_>>()
    };
    assert_eq!(summary(&loaded.list), summary(&list));
}

#[test]
fn the_file_format_is_plain_readable_json() {
    let loaded = model::load(Some(include_str!("fixtures/todo/todos.json")));

    assert_eq!(loaded.warning, None);
    let items: Vec<_> = loaded
        .list
        .items()
        .map(|t| (t.text.as_str(), t.done, t.priority))
        .collect();
    assert_eq!(
        items,
        [
            ("ship v2", false, Priority::High),
            ("no priority means normal", false, Priority::Normal),
            ("read a book", false, Priority::Low),
            ("buy milk", true, Priority::Normal),
        ]
    );
}

#[test]
fn a_missing_file_starts_empty_with_a_warning() {
    let loaded = model::load(None);

    assert_eq!(loaded.list, TodoList::default());
    assert!(loaded.warning.is_some());
}

#[test]
fn a_corrupt_file_starts_empty_with_a_warning_instead_of_failing() {
    for text in [
        "",
        "{ not json",
        "[1, 2, 3]",
        r#"{"items": [{"text": "x", "priority": "urgent"}]}"#,
        "\u{0}\u{1}binary",
    ] {
        let loaded = model::load(Some(text));

        assert_eq!(loaded.list, TodoList::default(), "{text:?}");
        let warning = loaded.warning.unwrap_or_else(|| panic!("{text:?}"));
        assert!(warning.contains("corrupt"), "{warning}");
    }
}

#[test]
fn blank_entries_in_the_file_are_dropped() {
    let loaded = model::load(Some(
        r#"{"items": [{"text": "  "}, {"text": " keep me "}]}"#,
    ));

    assert_eq!(texts(&loaded.list), ["keep me"]);
}

#[test]
fn ids_stay_unique_after_loading() {
    let mut list = TodoList::default();
    add(&mut list, "one", Priority::Normal);
    add(&mut list, "two", Priority::Normal);
    let mut list = model::load(Some(&list.to_json())).list;

    let three = add(&mut list, "three", Priority::Normal);
    list.apply(Command::Delete(three));

    assert_eq!(texts(&list), ["one", "two"]);
}

#[test]
fn blank_text_is_never_stored() {
    let mut list = TodoList::default();
    assert_eq!(
        list.apply(Command::Add {
            text: "   ".into(),
            priority: Priority::Normal
        }),
        None
    );
    let taxes = add(&mut list, "file taxes", Priority::Normal);

    let edited = list.apply(Command::Edit {
        id: taxes,
        text: "".into(),
    });

    assert_eq!(edited, None);
    assert_eq!(list.get(taxes).expect("kept").text, "file taxes");
    assert_eq!(model::view_state(&list).label, "1");
}
