/** 
 * @typedef {Object} JsonEvent
 * @property {number} id
 * @property {string} name
 */

/**
 * @typedef {Object} Tile
 * @property {string} region
 * @property {boolean} is_indoor
 */

/**
 * @typedef {Tile & JsonEvent} TileEvent
 */

/** 
 * @typedef {Object<number,TileEvent|JsonEvent>} JsonEvents
 */

/** 
 * @typedef {Object} Message
 * @property {number} timestamp
 * @property {string} message
 */

/** 
 * @typedef {Object} SnesEvent
 * @property {number} timestamp
 * @property {number?} tile_id
 * @property {number?} location_id
 * @property {number?} item_id
 * @property {number?} event_id
 * @property {number?} action_id
 * @property {number?} command_id
 */

/** 
 * @typedef {Object} RunObjectives
 * @property {boolean} finalized
 * @property {number} start_tile
 * @property {SnesEvent[]} objectives
 */

