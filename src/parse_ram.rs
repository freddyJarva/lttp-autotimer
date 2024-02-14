use std::collections::VecDeque;

use chrono::{DateTime, Utc};

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
    write::CsvWriter,
};

pub fn check_for_location_checks<W>(
    ram: &SnesRam,
    ram_history: &mut VecDeque<SnesRam>,
    checks: &mut Vec<Check>,
    writer: &mut W,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
    should_print: bool
) -> anyhow::Result<()>
where
    W: CsvWriter,
{
    for check in checks {
        match &check.conditions {
            Some(conditions) => {
                if conditions
                    .iter()
                    .all(|c| match_condition(c, events, ram, ram_history))
                {
                    check.mark_as_checked(time_of_read);
                    if should_print {
                        print.location_check(check);
                    }

                    let location_check_event = EventEnum::LocationCheck(check.clone());
                    writer.write_event(Event::from(&location_check_event))?;
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
                        if should_print {
                            print.location_check(check);
                        }
                        let location_check_event = EventEnum::LocationCheck(check.clone());
                        writer.write_event(Event::from(&location_check_event))?;
                        events.push(location_check_event);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn check_for_item_checks<W>(
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
    checks: &mut Vec<Check>,
    writer: &mut W,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
    should_print: bool
) -> anyhow::Result<()>
where
    W: CsvWriter,
{
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
                    writer.write_event(Event::from(&occurred_check))?;
                    events.push(occurred_check);
                    if should_print {
                        print.item_check(check);
                    }
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
                        if should_print {
                            print.item_check(check);
                        }

                        let item_event = EventEnum::ItemGet(check.clone());
                        writer.write_event(Event::from(&item_event))?;
                        events.push(item_event);
                    } else if check.is_progressive && current_check_value > check.snes_value {
                        check.progress_item(current_check_value, time_of_read);
                        if should_print {
                            print.item_check(check);
                        }

                        let item_event = EventEnum::ItemGet(check.clone());
                        writer.write_event(Event::from(&item_event))?;
                        events.push(item_event);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn check_for_transitions<W>(
    ram: &SnesRam,
    writer: &mut W,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
    should_print: bool,
) -> anyhow::Result<()>
where
    W: CsvWriter,
{
    // Use events if one transition has been triggered.
    match events.latest_transition() {
        Some(previous_transition) => {
            if let Ok(mut current_tile) = Tile::try_from_ram(ram, &previous_transition) {
                if current_tile.name != previous_transition.name {
                    current_tile.time_transit(time_of_read);
                    if should_print {
                        print.transition(&current_tile);
                    }
                    let transition_event = EventEnum::Transition(current_tile);
                    writer.write_event(Event::from(&transition_event))?;
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


pub fn check_for_events<W>(
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
    subscribed_events: &mut Vec<Check>,
    writer: &mut W,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
    should_print: bool,
) -> anyhow::Result<bool>
where
    W: CsvWriter,
{
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
                    writer.write_event(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                    if should_print {
                        print.event(event);
                    }
                    return Ok(event.id != 0 && event.id != 15);
                }
            }
            None => {
                if !event.is_progressive
                    && current_event_value & event.sram_mask.unwrap_or_default() != 0
                    && !event.is_checked
                {
                    event.mark_as_checked(time_of_read);
                    if should_print {
                        print.event(event);
                    }
                    let occurred_event = EventEnum::Other(event.clone());
                    writer.write_event(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                } else if event.is_progressive && current_event_value > event.snes_value {
                    event.progress_item(current_event_value, time_of_read);
                    if should_print {
                        print.event(event);
                    }
                    let occurred_event = EventEnum::Other(event.clone());
                    writer.write_event(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                    // Save & Quit and Reset will pause checks from occurring until player has gone in-game once more
                    return Ok(event.id != 0 && event.id != 15);
                }
            }
        }
    }

    Ok(true)
}

pub fn check_for_actions<W>(
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
    actions: &mut Vec<Check>,
    writer: &mut W,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
    should_print: bool
) -> anyhow::Result<()>
where
    W: CsvWriter,
{
    for event in actions {
        let current_event_value = ram.get_byte(event.sram_offset.unwrap_or_default() as usize);
        match &event.conditions {
            Some(conditions) => {
                if conditions
                    .iter()
                    .all(|condition| match_condition(condition, events, ram, previous_values))
                {
                    // All actions are considered progressive in essence
                    event.progress_item(current_event_value, time_of_read);

                    let occurred_event = EventEnum::Action(event.clone());
                    writer.write_event(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                    if should_print {
                        print.action(event);
                    }
                }
            }
            None => {}
        }
    }
    Ok(())
}

pub fn check_for_commands<W>(
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
    subscribed_commands: &mut Vec<Check>,
    writer: &mut W,
    commands: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
    should_print: bool,
) -> anyhow::Result<Option<Check>>
where
    W: CsvWriter,
{
    for command in subscribed_commands {
        let current_command_value = ram.get_byte(command.sram_offset.unwrap_or_default() as usize);
        match &command.conditions {
            Some(conditions) => {
                if conditions
                    .iter()
                    .all(|condition| match_condition(condition, commands, ram, previous_values))
                {
                    // All commands are considered progressive in essence
                    command.progress_item(current_command_value, time_of_read);

                    let occurred_command = EventEnum::Command(command.clone());
                    writer.write_event(Event::from(&occurred_command))?;
                    commands.push(occurred_command);
                    if should_print {
                        print.command(command);
                    }
                    return Ok(Some(command.to_owned()));
                }
            }
            None => {}
        }
    }
    Ok(None)
}

pub fn check_for_segment_run_start(
    ram: &SnesRam,
    ram_history: &mut VecDeque<SnesRam>,
    start_event: &EventEnum,
    events: &mut EventTracker
) -> anyhow::Result<bool>
{
    match start_event {
        EventEnum::Transition(tile) => {
            match events.latest_transition() {
                Some(previous_transition) => {
                    if let Ok(current_tile) = Tile::try_from_ram(ram, &previous_transition) {
                        if current_tile.name != previous_transition.name && current_tile.name == tile.name {
                            return Ok(true)
                        }
                    }
                }
                None => {
                    panic!("You've reached the unreachable, as EventTracker should always contain a transition when using ::new");
                }
            }
        }
        EventEnum::LocationCheck(check) => {
            if let Some(ref conditions) = check.conditions {
                if conditions.iter().all(|condition| match_condition(condition, events, ram, ram_history)) {
                    return Ok(true)
                }
            } else {
                let current_check_value =
                    ram.get_byte(check.sram_offset.unwrap_or_default() as usize);
                if ram_history.len() > 0
                    && (ram_history[ram_history.len() - 1]
                        .get_byte(check.sram_offset.unwrap_or_default() as usize)
                        != current_check_value)
                {
                    if current_check_value & check.sram_mask.unwrap_or_default() != 0
                    {
                        return Ok(true);
                    }
                }
            }
        },
        EventEnum::ItemGet(check) => {
            if let Some(ref conditions) = check.conditions {
                if conditions.iter().all(|condition| match_condition(condition, events, ram, ram_history)) {
                    return Ok(true)
                }
            }
        },
        EventEnum::Other(check) => {
            if let Some(ref conditions) = check.conditions {
                if conditions.iter().all(|condition| match_condition(condition, events, ram, ram_history)) {
                    return Ok(true)
                }
            }
        },
        EventEnum::Action(check) => {
            if let Some(ref conditions) = check.conditions {
                if conditions.iter().all(|condition| match_condition(condition, events, ram, ram_history)) {
                    return Ok(true)
                }
            }
        },
        EventEnum::Command(_) => panic!("EventEnum command should not be able to be part of segment objectives"),
    }
    Ok(false)
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
        Conditions::ValueChanged {
            sram_offset,
            sram_mask,
        } => ram_value_change_condition_met(previous_values, ram, sram_offset, sram_mask),
        Conditions::ValueChangedTo {
            sram_offset,
            sram_value,
            sram_mask,
        } => {
            ram.get_byte(*sram_offset) & sram_mask == *sram_value & sram_mask
                && ram_value_change_condition_met(previous_values, ram, sram_offset, sram_mask)
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
        Conditions::PreviousAction { id } => {
            events.latest_action().map(|e| e.id == *id).unwrap_or(false)
        }
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
            sram_mask,
        } => ram.get_byte(*sram_offset) & sram_mask == *sram_value & sram_mask,
        Conditions::CheckMade { id } => events.find_location_check(*id).is_some(),
        Conditions::HasItem { id } => !events.items_with_id(*id).is_empty(),
        Conditions::PreviousValueEq {
            sram_offset,
            sram_value,
            sram_mask,
        } => {
            if previous_values.len() > 0 {
                let previous_ram = &previous_values[previous_values.len() - 1];
                (previous_ram.get_byte(*sram_offset) & sram_mask) == *sram_value & sram_mask
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
        Conditions::All { subconditions } => subconditions
            .iter()
            .all(|subcondition| match_condition(subcondition, events, ram, previous_values)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{snes::SnesRamInitializer, tile::deserialize_transitions};

    struct MockCsvWriter;

    impl CsvWriter for MockCsvWriter {
        fn write_event<S>(&mut self, _: S) -> anyhow::Result<()>
        where
            S: serde::Serialize,
        {
            Ok(())
        }
    }

    fn get_coordinate_condition(tile: &Tile) -> Option<Conditions> {
        match &tile.conditions {
            Some(conditions) => conditions
                .iter()
                .filter(|&c| match c {
                    Conditions::Coordinates { coordinates: _ } => true,
                    _ => false,
                })
                .map(|c| c.clone())
                .next()
                .clone(),
            None => None,
        }
    }

    fn point_from_coordinate_condition(condition: Conditions) -> Vec<(u16, u16)> {
        match condition {
            Conditions::Coordinates { coordinates } => {
                let mut points: Vec<(u16, u16)> = vec![];
                for coordinate in coordinates {
                    match coordinate {
                        crate::condition::Coordinate::Pair { x, y } => points.push((x, y)),
                        crate::condition::Coordinate::Range { x, y } => points.push((x.0, y.0)),
                        crate::condition::Coordinate::CRange { x, y } => points.push((x.0, y.0)),
                        crate::condition::Coordinate::Chest { x, y } => points.push((x, y)),
                        crate::condition::Coordinate::BigChest { x, y } => points.push((x, y)),
                        crate::condition::Coordinate::Stairs { x, y } => points.push((x, y)),
                    };
                }
                points
            }
            _ => panic!("Filter out non-coordinate conditions before calling this!"),
        }
    }

    #[test]
    fn test_tile_coordinate_conditions() {
        // If there's overlap in coordinate conditions,
        // i.e. 2 or more transitions could be triggered from the same snes ram values,
        // then this test should fail

        // Given
        let mut mock_writer = MockCsvWriter {};
        let mut printer = StdoutPrinter::new(false);
        for tile in deserialize_transitions().unwrap() {
            if let Some(coordinate_condition) = get_coordinate_condition(&tile) {
                let points = point_from_coordinate_condition(coordinate_condition);
                let address_values = tile.address_value;
                for address in address_values {
                    for point in &points {
                        let ram = SnesRamInitializer {
                            transition_x: Some(point.0),
                            transition_y: Some(point.1),
                            entrance_id: Some(address as u8),
                            indoors: Some(if tile.indoors { 1 } else { 0 }),
                            ..Default::default()
                        }
                        .build();
                        let mut events = EventTracker::new();
                        // When
                        check_for_transitions(
                            &ram,
                            &mut mock_writer,
                            &mut events,
                            &mut printer,
                            &Utc::now(),
                            true
                        )
                        .unwrap();
                        // Then
                        let latest_event = events.latest_transition().unwrap();
                        assert_attrs! {latest_event: id == tile.id,}
                    }
                }
            }
        }
    }
}
