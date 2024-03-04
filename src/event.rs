use chrono::serde::ts_milliseconds;
use std::borrow::Borrow;

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;

use crate::{check::Check, tile::Tile};

pub trait EventLog {
    fn latest_transition(&self) -> Option<Tile>;
    fn latest_location_check(&self) -> Option<Check>;
    fn latest_item_get(&self) -> Option<Check>;
    fn latest_other_event(&self) -> Option<Check>;
    fn latest_action(&self) -> Option<Check>;
    fn latest_objective(&self) -> Option<EventEnum>;
    fn objectives_between(&self, start: EventEnum, end: Option<EventEnum>) -> Vec<EventEnum>;
    fn find_other_event(&self, id: usize) -> Option<Check>;
    fn find_location_check(&self, id: usize) -> Option<Check>;
    fn find_latest_command_by(&self, id: usize) -> Option<Check>;
    fn others_with_id(&self, id: usize) -> Vec<Check>;
    fn items_with_id(&self, id: usize) -> Vec<Check>;
    fn location_checks_with_id(&self, id: usize) -> Vec<Check>;
}

impl EventLog for EventTracker {
    fn latest_transition(&self) -> Option<Tile> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                if let EventEnum::Transition(_) = event {
                    true
                } else {
                    false
                }
            })
            .map(|event| {
                if let EventEnum::Transition(t) = event {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn latest_location_check(&self) -> Option<Check> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                if let EventEnum::LocationCheck(_) = event {
                    true
                } else {
                    false
                }
            })
            .map(|event| {
                if let EventEnum::LocationCheck(t) = event {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn latest_item_get(&self) -> Option<Check> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                if let EventEnum::ItemGet(_) = event {
                    true
                } else {
                    false
                }
            })
            .map(|event| {
                if let EventEnum::ItemGet(t) = event {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn latest_other_event(&self) -> Option<Check> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                if let EventEnum::Other(_) = event {
                    true
                } else {
                    false
                }
            })
            .map(|event| {
                if let EventEnum::Other(t) = event {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn latest_action(&self) -> Option<Check> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                if let EventEnum::Action(_) = event {
                    true
                } else {
                    false
                }
            })
            .map(|event| {
                if let EventEnum::Action(t) = event {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn latest_objective(&self) -> Option<EventEnum> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                match event {
                    EventEnum::Transition(_) => true,
                    EventEnum::LocationCheck(_) => true,
                    EventEnum::ItemGet(_) => true,
                    EventEnum::Other(_) => true,
                    EventEnum::Action(_) => false,
                    EventEnum::Command(_) => false,
                    EventEnum::Composite(_) => panic!("Composites shouldn't exist in EventLog"),
                }
            }).cloned()
    }

    fn find_other_event(&self, id: usize) -> Option<Check> {
        self.log
            .iter()
            .find(|&check| {
                if let EventEnum::Other(check) = check {
                    check.id == id
                } else {
                    false
                }
            })
            .map(|check| {
                if let EventEnum::Other(t) = check {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn find_location_check(&self, id: usize) -> Option<Check> {
        self.log
            .iter()
            .find(|&check| {
                if let EventEnum::LocationCheck(check) = check {
                    check.id == id
                } else {
                    false
                }
            })
            .map(|check| {
                if let EventEnum::LocationCheck(t) = check {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn find_latest_command_by(&self, id: usize) -> Option<Check> {
        self.log
            .iter()
            .rev()
            .find(|&check| {
                if let EventEnum::Command(check) = check {
                    check.id == id
                } else {
                    false
                }
            })
            .map(|check| {
                if let EventEnum::Command(t) = check {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }


    fn others_with_id(&self, id: usize) -> Vec<Check> {
        self.log
            .iter()
            .filter(|&check| match check {
                EventEnum::Other(check) => check.id == id,
                _ => false,
            })
            .map(|check| {
                if let EventEnum::Other(t) = check {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
            .collect()
    }

    fn items_with_id(&self, id: usize) -> Vec<Check> {
        self.log
            .iter()
            .filter(|&check| match check {
                EventEnum::ItemGet(check) => check.id == id,
                _ => false,
            })
            .map(|check| {
                if let EventEnum::ItemGet(t) = check {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
            .collect()
    }

    fn location_checks_with_id(&self, id: usize) -> Vec<Check> {
        self.log
            .iter()
            .filter(|&check| match check {
                EventEnum::LocationCheck(check) => check.id == id,
                _ => false,
            })
            .map(|check| {
                if let EventEnum::LocationCheck(t) = check {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
            .collect()
    }

    fn objectives_between(&self, start: EventEnum, end: Option<EventEnum>) -> Vec<EventEnum> {
        let event_iter = self.log
                .iter()
                .skip_while(|&event| !event.eq(&start))
                .filter(|&event| match event {
                    EventEnum::Action(_) | EventEnum::Command(_) => false,
                    _ => true
                });
        if let Some(end) = end {
            return event_iter
                .take_while(|&event| !event.eq(&end))
                .cloned().collect()
        }
        event_iter.cloned().collect()
    }
}

pub trait EventCompactor {
    fn compact(self) -> Self;
}

impl EventCompactor for Vec<EventEnum> {
    fn compact(self) -> Self {
        if self.len() == 0 {
            return self
        }
        let mut previous_val: Option<Check> = None;
        let mut new: Vec<EventEnum> = Vec::new();
        for event in self {
            match event {
                EventEnum::Command(_) => (),
                EventEnum::LocationCheck(ref check) | EventEnum::ItemGet(ref check) | EventEnum::Other(ref check) => {
                    if let Some(previous_check) = previous_val {
                        let previous_time = previous_check.time_of_check.expect("previous should have a timestamp when running compact");
                        if previous_time == check.time_of_check.expect("check should have a timestamp when running compact") {
                            new.pop();
                            new.push(EventEnum::Composite((format!("{} & {}", previous_check.name, check.name), previous_check, check.clone())));
                        } else {
                            new.push(event.clone());
                        }
                    } else {
                        println!("Pushing {} - not a composite", event.name());
                        new.push(event.clone())
                    }
                    previous_val = Some(check.clone());
                },
                EventEnum::Action(_) | EventEnum::Transition(_) | EventEnum::Composite(_) => {
                    new.push(event);
                    previous_val = None;
                }
            }
        }
        new
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventEnum {
    Transition(Tile),
    LocationCheck(Check),
    ItemGet(Check),
    Other(Check),
    Action(Check),
    Command(Check),
    Composite((String, Check, Check)),
}

impl EventEnum {
    pub fn name(&self) -> &str {
        match self {
            EventEnum::Transition(t) => &t.name,
            EventEnum::LocationCheck(c) => &c.name,
            EventEnum::ItemGet(c) => &c.name,
            EventEnum::Other(c) => &c.name,
            EventEnum::Action(c) => &c.name,
            EventEnum::Command(c) => &c.name,
            EventEnum::Composite(c) => &c.1.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandState {
    StartRecording,
    RecordingInProgress(Check),
    RunStarted(usize),
    SegmentRecorded,
    RunFinished,
    ClearEventLog,
    None
}

pub struct EventTracker {
    log: Vec<EventEnum>,
}

impl EventTracker {
    /// Sets an initial 'auto timer start' Transition with values of `0`.
    ///
    /// This is necessary for the absolute first transition check to work,
    /// as it needs a value to compare to to see if a transition triggered.
    pub fn new() -> Self {
        Self {
            log: vec![EventEnum::Transition(Tile {
                id: 9000,
                name: "AUTO_TIMER_START".to_string(),
                address_value: vec![0x0],
                timestamp: Some(Utc::now()),
                indoors: false,
                conditions: None,
                region: "START".to_string(),
            })],
        }
    }

    pub fn push(&mut self, event: EventEnum) {
        self.log.push(event)
    }
}

impl From<Vec<EventEnum>> for EventTracker {
    fn from(log: Vec<EventEnum>) -> Self {
        EventTracker { log }
    }
}

/// Struct used for serializing different types of checks into the same csv format.
/// Events include transitions, checking locations (e.g. chests), and getting items
#[derive(Serialize, Debug, PartialEq)]
pub struct Event {
    #[serde(with = "ts_milliseconds")]
    timestamp: DateTime<Utc>,
    #[serde(skip_serializing)]
    indoors: Option<bool>,
    tile_id: Option<usize>,
    location_id: Option<usize>,
    item_id: Option<usize>,
    event_id: Option<usize>,
    action_id: Option<usize>,
    command_id: Option<usize>,
}

impl From<&Tile> for Event {
    fn from(transition: &Tile) -> Self {
        Event {
            timestamp: transition
                .timestamp
                .expect("Found transition missing timestamp when serializing"),
            indoors: Some(transition.indoors),
            tile_id: Some(transition.id),
            location_id: None,
            item_id: None,
            ..Default::default()
        }
    }
}

impl From<&mut Tile> for Event {
    fn from(transition: &mut Tile) -> Self {
        Event {
            timestamp: transition
                .timestamp
                .expect("Found transition missing timestamp when serializing"),
            indoors: Some(transition.indoors),
            tile_id: Some(transition.id),
            location_id: None,
            item_id: None,
            ..Default::default()
        }
    }
}

impl<T> From<T> for Event
where
    T: Borrow<Check>,
{
    fn from(check: T) -> Self {
        let check: &Check = check.borrow();
        let timestamp = check
            .time_of_check
            .expect("Found check missing timestamp when serializing");
        if check.is_item {
            Event {
                timestamp,
                item_id: Some(check.id),
                ..Default::default()
            }
        } else {
            Event {
                timestamp,
                location_id: Some(check.id),
                ..Default::default()
            }
        }
    }
}

impl From<&EventEnum> for Event {
    fn from(event: &EventEnum) -> Self {
        match event {
            EventEnum::Transition(t) => Event::from(t),
            EventEnum::LocationCheck(check) => Event {
                timestamp: check.time_of_check.unwrap(),
                location_id: Some(check.id),
                ..Default::default()
            },
            EventEnum::ItemGet(check) => Event {
                timestamp: check.time_of_check.unwrap(),
                item_id: Some(check.id),
                ..Default::default()
            },
            EventEnum::Other(check) => Event {
                timestamp: check.time_of_check.unwrap(),
                event_id: Some(check.id),
                ..Default::default()
            },
            EventEnum::Action(check) => Event {
                timestamp: check.time_of_check.unwrap(),
                action_id: Some(check.id),
                ..Default::default()
            },
            EventEnum::Command(check) => Event {
                timestamp: check.time_of_check.unwrap(),
                command_id: Some(check.id),
                ..Default::default()
            },
            EventEnum::Composite(_) => panic!("Composite EventEnum shouldn't be turned into Event for writing"),
        }
    }
}

impl Default for Event {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc.timestamp_millis_opt(0).unwrap(),
            indoors: Default::default(),
            tile_id: Default::default(),
            location_id: Default::default(),
            item_id: Default::default(),
            event_id: Default::default(),
            action_id: Default::default(),
            command_id: Default::default(),
        }
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {

    use crate::assert_attrs;

    use super::*;

    macro_rules! convert_to_event {
        ($($name:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (from_struct, expected) = $values;
                    assert_eq!(Event::from(&from_struct), expected)
                }
            )*
        };
    }

    convert_to_event! {
        from_location_check: (
            Check {
                id: 0,
                name: "Mushroom".to_string(),
                sram_offset: Some(0x411),
                sram_mask: Some(0x10),
                time_of_check: Some(Utc.timestamp_millis_opt(200).unwrap()),
                ..Default::default()
            },
            Event {
                location_id: Some(0),
                timestamp: Utc.timestamp_millis_opt(200).unwrap(),
                ..Default::default()
            }
        ),
        from_normal_item_check: (
            Check {
                id: 4,
                name: "Hookshot".to_string(),
                sram_offset: Some(0x342),
                sram_mask: Some(0x01),
                time_of_check: Some(Utc.timestamp_millis_opt(200).unwrap()),
                is_item: true,
                ..Default::default()
            },
            Event {
                item_id: Some(4),
                timestamp: Utc.timestamp_millis_opt(200).unwrap(),
                ..Default::default()
            }
        ),
        from_progressive_item_check: (
            Check {
                id: 27,
                name: "Progressive Sword".to_string(),
                sram_offset: Some(0x342),
                sram_mask: Some(0x01),
                time_of_check: Some(Utc.timestamp_millis_opt(200).unwrap()),
                is_item: true,
                is_progressive: true,
                progressive_level: 3,
                ..Default::default()
            },
            Event {
                item_id: Some(27),
                timestamp: Utc.timestamp_millis_opt(200).unwrap(),
                ..Default::default()
            }
        ),
        from_transition: (
            Tile {
                id: 1337,
                region: "A great region".to_string(),
                timestamp: Some(Utc.timestamp_millis_opt(200).unwrap()),
                ..Default::default()
            },
            Event {
                tile_id: Some(1337),
                timestamp: Utc.timestamp_millis_opt(200).unwrap(),
                indoors: Some(false),
                ..Default::default()
            }
        ),
    }

    fn event_log(extra_event: Option<(usize, EventEnum)>) -> Vec<EventEnum> {
        let mut log = vec![
            EventEnum::ItemGet(Check {
                id: 0,
                ..Default::default()
            }),
            EventEnum::Transition(Tile {
                id: 1,
                ..Default::default()
            }),
            EventEnum::LocationCheck(Check {
                id: 2,
                ..Default::default()
            }),
            EventEnum::Transition(Tile {
                id: 3,
                ..Default::default()
            }),
            EventEnum::LocationCheck(Check {
                id: 4,
                ..Default::default()
            }),
            EventEnum::ItemGet(Check {
                id: 5,
                ..Default::default()
            }),
            EventEnum::Action(Check {
                id: 6,
                ..Default::default()
            }),
        ];
        if let Some((idx, event)) = extra_event {
            log.insert(idx, event)
        };
        log
    }

    macro_rules! test_eventlog {
        ($($name:ident: $function:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (event_log, expected) = $values;
                    let event_tracker = EventTracker::from(event_log);
                    assert_eq!(event_tracker.$function(), expected)
                }
            )*
        };
    }

    test_eventlog! {
        latest_transition: latest_transition: (event_log(None), Some(Tile {
            id: 3,
            ..Default::default()
        })),
        latest_location_check: latest_location_check: (event_log(None), Some(Check::new(4))),
        latest_item_get: latest_item_get: (event_log(None), Some(Check::new(5))),
        latest_action: latest_action: (event_log(None), Some(Check::new(6))),
        GIVEN_no_transitions_THEN_return_none: latest_transition: (vec![], None),
        GIVEN_no_location_checks_THEN_return_none: latest_location_check: (vec![], None),
        GIVEN_no_item_get_THEN_return_none: latest_item_get: (vec![], None),
        GIVEN_no_action_THEN_return_none: latest_item_get: (vec![], None),
    }

    macro_rules! test_eventlog_with_param {
        ($($name:ident: $function:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (event_log, param, expected) = $values;
                    let event_tracker = EventTracker::from(event_log);
                    match expected {
                        Some::<usize>(expected_id) => {
                            let actual = event_tracker.$function(param).unwrap();
                            assert_attrs! {
                                actual: id == expected_id,
                            }
                        }
                        None => assert_eq!(event_tracker.$function(param), None)
                    }
                }
            )*
        };
    }

    test_eventlog_with_param! {
        find_other_event: find_other_event: (
            event_log(Some((3, EventEnum::Other(Check::new(16))))),
            16,
            Some(16)
        ),
        GIVEN_no_events_THEN_return_none: find_other_event: (vec![], 12, None),
        GIVEN_no_event_of_type_other_with_given_idx_THEN_return_None: find_other_event: (event_log(None), 2, None),
        find_location_check: find_location_check: (
            event_log(Some((5, EventEnum::LocationCheck(Check::new(31))))),
            31,
            Some(31)
        ),
        GIVEN_no_locations_THEN_return_none: find_location_check: (vec![], 12, None),
        GIVEN_no_event_of_type_location_with_given_idx_THEN_return_None: find_location_check: (event_log(None), 0, None),
    }

    macro_rules! test_eventlog_find_alls {
        ($($name:ident: $function:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (event_log, param, expected_size) = $values;
                    let event_tracker = EventTracker::from(event_log);

                    let actual_list = event_tracker.$function(param);
                    assert_eq!(actual_list.len(), expected_size);
                }
            )*
        };
    }

    test_eventlog_find_alls! {
        find_all_other_events_with_id: others_with_id: (
            event_log(Some((3, EventEnum::Other(Check::new(16))))),
            16,
            1
        ),
        find_all_items_with_id: items_with_id: (
            event_log(Some((3, EventEnum::ItemGet(Check::new(0))))),
            0,
            2
        ),
        find_all_checks_with_id: location_checks_with_id: (
            event_log(Some((3, EventEnum::LocationCheck(Check::new(4))))),
            4,
            2
        ),
        GIVEN_no_events_with_id_THEN_return_empty_vec: others_with_id: (event_log(None), 999, 0),
        GIVEN_no_items_with_id_THEN_return_empty_vec: items_with_id: (event_log(None), 999, 0),
        GIVEN_no_checks_with_id_THEN_return_empty_vec: location_checks_with_id: (event_log(None), 999, 0),
    }

    #[test]
    fn new_event_tracker_is_initialized_with_start_transition() {
        let last_transition = EventTracker::new().latest_transition().unwrap();
        assert_attrs! {
            last_transition: address_value == [0x0],
            name == "AUTO_TIMER_START",
            indoors == false,
        };
    }
}
