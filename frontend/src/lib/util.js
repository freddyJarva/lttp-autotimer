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
    o.forEach(function(val) {
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
 * sums up numbers in a list
 *
 * @param {number[]} nums
 * @returns {number}
 */
function sum(...nums) {
    return nums.reduce((a, b) => a + b, 0)
}

/**
 * Find best run from a list of runs
 *
 * @param {number[][]} runTimes
 * @returns {number[]?} best run from the list, or null if the list is empty
 */
function getBestRun(runTimes) {
    if (runTimes.length === 0 || runTimes[0].length === 0) {
        return null
    }
    let /** @type {number[] | null} */ best = null
    for (let i = 0; i < runTimes[0].length; i++) {
        let run = []
        for (let j = 0; j < runTimes.length; j++) {
            run.push(runTimes[j][i])
        }
        if (best === null) {
            best = run
            continue
        }
        if (sum(...run) < sum(...best)) {
            best = run
        }
    }
    return best
}

/**
 * Parses snes event and returns info
 *
 * @param {SnesEvent} e - event data returned from rust
 * @returns {JsonEvent?}
 */
function eventInfo(e) {
    if (e.tile_id !== null) {
        return tiles[e.tile_id];
    }
    if (e.item_id !== null) {
        return items[e.item_id];
    }
    if (e.event_id !== null) {
        return events[e.event_id];
    }
    if (e.location_id !== null) {
        return checks[e.location_id];
    }
    return null;
}

/**
 * @param {SnesEvent?} l
 * @param {SnesEvent?} r
 * @returns {boolean}
 */
function isDuplicateEvent(l, r) {
    if (l === null || r === null) {
        return false
    }
    return l.timestamp === r.timestamp
        && l.event_id === r.event_id
        && l.location_id === r.location_id
        && l.item_id === r.item_id
        && l.tile_id === r.tile_id
}

/**
 * Checks if the two events are triggers for the same objective
 *
 * We only consider them a combined event if it's a check + item combo
 *
 * @param {SnesEvent?} l
 * @param {SnesEvent?} r
 * @returns {boolean}
 */
function isCombinedEvent(l, r) {
    if (l === null || r === null) {
        return false
    }
    if ((l.location_id === null && l.item_id === null)
        || (r.location_id === null && r.item_id === null)) {
        return false
    }
    return ((l.location_id !== null && r.item_id !== null)
        || (l.item_id !== null && r.location_id !== null))
        && l.timestamp === r.timestamp
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
        let info = eventInfo(o)
        console.log(`Objective with id ${o.tile_id} is ` + info?.name)
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
    fmtObjective,
    getBestRun,
    sum,
    isDuplicateEvent,
    isCombinedEvent,
}
