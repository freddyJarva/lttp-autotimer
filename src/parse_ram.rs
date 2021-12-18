use std::{collections::VecDeque, fs::File};

use chrono::{DateTime, Utc};
use csv::Writer;

use crate::{
    check::Check,
    condition::{
        coordinate_condition_met, current_tile_condition_met, dungeon_counter_condition_met,
        ram_value_change_condition_met, Conditions,
        Value::{CheckCount, EventCount, ItemCount, ValueOfAddress},
    },
    event::{Event, EventEnum, EventLog, EventTracker},
    output::StdoutPrinter,
    snes::{NamedAddresses, SnesRam},
    tile::Tile,
};

pub fn check_for_location_checks(
    ram: &SnesRam,
    ram_history: &mut VecDeque<SnesRam>,
    checks: &mut Vec<Check>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
) -> anyhow::Result<()> {
    for check in checks {
        match &check.conditions {
            Some(conditions) => {
                if conditions
                    .iter()
                    .all(|c| match_condition(c, events, ram, ram_history))
                {
                    check.mark_as_checked(time_of_read);
                    print.location_check(check);

                    let location_check_event = EventEnum::LocationCheck(check.clone());
                    writer.serialize(Event::from(&location_check_event))?;
                    events.push(location_check_event);
                }
            }
            None => {
                let current_check_value =
                    ram.get_byte(check.sram_offset.unwrap_or_default() as usize);
                if ram_history.len() > 0
                    && (ram_history[ram_history.len() - 1]
                        .get_byte(check.sram_offset.unwrap_or_default() as usize)
                        != current_check_value)
                {
                    if current_check_value & check.sram_mask.unwrap_or_default() != 0
                        && !check.is_checked
                    {
                        check.mark_as_checked(time_of_read);
                        print.location_check(check);
                        let location_check_event = EventEnum::LocationCheck(check.clone());
                        writer.serialize(Event::from(&location_check_event))?;
                        events.push(location_check_event);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn check_for_item_checks(
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
    checks: &mut Vec<Check>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
) -> anyhow::Result<()> {
    for check in checks {
        let current_check_value = ram.get_byte(check.sram_offset.unwrap_or_default() as usize);

        match &check.conditions {
            Some(conditions) => {
                if (check.is_progressive || !check.is_checked)
                    && conditions
                        .iter()
                        .all(|condition| match_condition(condition, events, ram, previous_values))
                {
                    if !check.is_progressive {
                        check.mark_as_checked(time_of_read)
                    } else {
                        check.progress_item(current_check_value, time_of_read)
                    }
                    let occurred_check = EventEnum::ItemGet(check.clone());
                    writer.serialize(Event::from(&occurred_check))?;
                    events.push(occurred_check);
                    print.item_check(check);
                }
            }
            None => {
                if previous_values.len() > 0
                    && (previous_values[previous_values.len() - 1]
                        .get_byte(check.sram_offset.unwrap_or_default() as usize)
                        != current_check_value)
                {
                    if !check.is_progressive
                        && current_check_value & check.sram_mask.unwrap_or_default() != 0
                        && !check.is_checked
                    {
                        check.mark_as_checked(time_of_read);
                        print.item_check(check);

                        let item_event = EventEnum::ItemGet(check.clone());
                        writer.serialize(Event::from(&item_event))?;
                        events.push(item_event);
                    } else if check.is_progressive && current_check_value > check.snes_value {
                        check.progress_item(current_check_value, time_of_read);
                        print.item_check(check);

                        let item_event = EventEnum::ItemGet(check.clone());
                        writer.serialize(Event::from(&item_event))?;
                        events.push(item_event);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn check_for_transitions(
    ram: &SnesRam,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
) -> anyhow::Result<()> {
    // Use events if one transition has been triggered.
    match events.latest_transition() {
        Some(previous_transition) => {
            if let Ok(mut current_tile) = Tile::try_from_ram(ram, &previous_transition) {
                if current_tile.name != previous_transition.name {
                    current_tile.time_transit(time_of_read);
                    print.transition(&current_tile);
                    let transition_event = EventEnum::Transition(current_tile);
                    writer.serialize(Event::from(&transition_event))?;
                    events.push(transition_event);
                }
            }
        }
        None => {
            panic!("You've reached the unreachable, as EventTracker should always contain a transition when using ::new");
        }
    }

    Ok(())
}

pub fn check_for_events(
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
    subscribed_events: &mut Vec<Check>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
) -> anyhow::Result<bool> {
    for event in subscribed_events {
        let current_event_value = ram.get_byte(event.sram_offset.unwrap_or_default() as usize);
        match &event.conditions {
            Some(conditions) => {
                if (event.is_progressive || !event.is_checked)
                    && conditions
                        .iter()
                        .all(|condition| match_condition(condition, events, ram, previous_values))
                {
                    if !event.is_progressive {
                        event.mark_as_checked(time_of_read)
                    } else {
                        event.progress_item(current_event_value, time_of_read)
                    }
                    let occurred_event = EventEnum::Other(event.clone());
                    writer.serialize(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                    print.event(event);
                    return Ok(event.id != 0 && event.id != 15);
                }
            }
            None => {
                if !event.is_progressive
                    && current_event_value & event.sram_mask.unwrap_or_default() != 0
                    && !event.is_checked
                {
                    event.mark_as_checked(time_of_read);
                    print.event(event);
                    let occurred_event = EventEnum::Other(event.clone());
                    writer.serialize(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                } else if event.is_progressive && current_event_value > event.snes_value {
                    event.progress_item(current_event_value, time_of_read);
                    print.event(event);
                    let occurred_event = EventEnum::Other(event.clone());
                    writer.serialize(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                    // Save & Quit and Reset will pause checks from occurring until player has gone in-game once more
                    return Ok(event.id != 0 && event.id != 15);
                }
            }
        }
    }

    Ok(true)
}

pub fn match_condition(
    condition: &Conditions,
    events: &EventTracker,
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
) -> bool {
    match condition {
        Conditions::PreviousTile(condition) => {
            let previous_tile = &events
                .latest_transition()
                .expect("Transition should always exist");
            current_tile_condition_met(condition, previous_tile)
        }
        Conditions::Coordinates { coordinates } => coordinate_condition_met(&coordinates, ram),
        Conditions::Underworld => ram.indoors() == 1,
        Conditions::DungeonCounterIncreased { sram_offset } => {
            dungeon_counter_condition_met(previous_values, ram, sram_offset)
        }
        Conditions::ValueChanged { sram_offset } => {
            ram_value_change_condition_met(previous_values, ram, sram_offset)
        }
        Conditions::CurrentTile(condition) => {
            let current_tile = &events
                .latest_transition()
                .expect("Transition should always exist");
            current_tile_condition_met(condition, current_tile)
        }
        Conditions::Any { subconditions } => subconditions
            .iter()
            .any(|subcondition| match_condition(subcondition, events, ram, previous_values)),
        Conditions::PreviousEvent { id } => events
            .latest_other_event()
            .map(|e| e.id == *id)
            .unwrap_or(false),
        Conditions::BitWiseTrue {
            sram_offset: _,
            sram_mask: _,
        } => todo!(),
        Conditions::Not { subconditions } => subconditions
            .iter()
            .all(|subcondition| !match_condition(subcondition, events, ram, previous_values)),
        Conditions::ValueEq {
            sram_offset,
            sram_value,
        } => ram.get_byte(*sram_offset) == *sram_value,
        Conditions::CheckMade { id } => events.find_location_check(*id).is_some(),
        Conditions::PreviousValueEq {
            sram_offset,
            sram_value,
        } => {
            if previous_values.len() > 0 {
                let previous_ram = &previous_values[previous_values.len() - 1];
                previous_ram.get_byte(*sram_offset) == *sram_value
            } else {
                false
            }
        }
        Conditions::ValueGreaterThan { sram_offset, other } => match other {
            ValueOfAddress(other_address) => {
                ram.get_byte(*sram_offset) > ram.get_byte(*other_address)
            }
            CheckCount(_) => todo!(),
            ItemCount(id) => ram.get_byte(*sram_offset) > events.items_with_id(*id).len() as u8,
            EventCount(id) => ram.get_byte(*sram_offset) > events.others_with_id(*id).len() as u8,
        },
    }
}
