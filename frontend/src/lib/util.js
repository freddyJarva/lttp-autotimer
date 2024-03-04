const FIXED_FRACTION = 2

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
    let end = run[run.length-1].timestamp
    return end - start
}

export {
    fmtDelta,
    fmTime,
    runTime,
}
