import tilesJson from '../events/tiles.json';
import eventJson from '../events/events.json';
import checksJson from '../events/checks.json';
import itemsJson from '../events/items.json';

const FIXED_FRACTION = 2

/**
 * @param {JsonEvent[] | TileEvent[]} o
 * @returns JsonEvents | Object<number, TileEvent>
 */
function toIdObjectMap(o) {
    let /** @type {JsonEvents | Object<number, TileEvent>} */ objects = {};
    o.forEach(function (val) {
        objects[val.id] = val;
    });
    return objects;
}

// @ts-ignore
const /** @type {Object<number, TileEvent>} */ tiles = toIdObjectMap(tilesJson);
const /** @type {JsonEvents} */ events = toIdObjectMap(eventJson);
const /** @type {JsonEvents} */ checks = toIdObjectMap(checksJson);
const /** @type {JsonEvents} */ items = toIdObjectMap(itemsJson);

/**
 * Take two unix times with millisecond precision and format them.
 *
 * @param {number} current
 * @param {number?} previous - previous time, defaulting to `current` if null (more convenient for our usage).
 * @returns {string} string represantation in format S.fff
 */
function fmtDelta(current, previous) {
    let delta = current - (previous ?? current)
    if (delta < 1) {
        return (0).toFixed(FIXED_FRACTION)
    }
    return (delta / 1000).toFixed(FIXED_FRACTION)
}

/**
 * Format time in milliseconds
 *
 * @param {number} timeMilli - time to format
 * @returns {string} string representation of `timeMilli` in format S.fff
 */
function fmTime(timeMilli) {
    return (timeMilli / 1000).toFixed(FIXED_FRACTION)
}

/**
 * Calculate total time of the given run
 *
 * @param {SnesEvent[]} run
 * @returns {number?} elapsed time of run in millisecond precision, or null if not a proper run
 */
function runTime(run) {
    if (run.length < 2) {
        return null
    }
    let start = run[0].timestamp
    let end = run[run.length - 1].timestamp
    return end - start
}

/**
 * Parses snes event and returns info
 *
 * @param {SnesEvent} e - event data returned from rust
 * @returns {JsonEvent?}
 */
function eventInfo(e) {
    if (e.tile_id) {
        return tiles[e.tile_id];
    }
    if (e.item_id) {
        return items[e.item_id];
    }
    if (e.event_id) {
        return events[e.event_id];
    }
    if (e.location_id) {
        return checks[e.location_id];
    }
    return null;
}

/**
 * @param {SnesEvent} o
 * @returns {string} string representation the objective 
 */
function fmtObjective(o) {
    let prefix = ''
    if (o.item_id) {
        prefix = 'Get'
    } else if (o.tile_id) {
        prefix = 'Go to'
    } else if (o.location_id) {
        prefix = 'Check'
    }
    return `${prefix} ${eventInfo(o)?.name ?? 'I am Error'}`
}

export {
    fmtDelta,
    fmTime,
    runTime,
    tiles,
    events,
    checks,
    items,
    eventInfo,
    fmtObjective
}
