<script>
	/// <reference path="../typedefs.js" />
	import '$lib/styles.css';

	import { onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { invoke } from '@tauri-apps/api/tauri';
	import tilesJson from '../events/tiles.json';
	import eventJson from '../events/events.json';
	import checksJson from '../events/checks.json';
	import itemsJson from '../events/items.json';
	import { fmtDelta, runTime, fmTime } from '$lib/util';
	import Timer from './timer.svelte';

	/**
	 * @param {JsonEvent[]} o
	 * @returns {JsonEvents}
	 */
	function toIdObjectMap(o) {
		let /** @type {JsonEvents} */ objects = {};
		o.forEach(function (/** @type {JsonEvent} */ val) {
			objects[val.id] = val;
		});
		return objects;
	}

	let /** @type {JsonEvents} */ tiles = toIdObjectMap(tilesJson);
	let /** @type {JsonEvents} */ events = toIdObjectMap(eventJson);
	let /** @type {JsonEvents} */ checks = toIdObjectMap(checksJson);
	let /** @type {JsonEvents} */ items = toIdObjectMap(itemsJson);

	let unlisten_snes_events;

	let isRecording = false;

	let /** @type {SnesEvent[]} */ snesEvents = [];

	let current_tile = -1;
	let /** @type {RunObjectives} */ runObjectives = {
			start_tile: current_tile,
			objectives: [],
			finalized: false
		};
	let /** @type {SnesEvent[]} */ currentRun = [];
	let currentObjective = 0;
	let runStarted = false;
	let runFinished = false;

	/** @param {string} cmd */
	async function sendTimerCommand(cmd) {
		invoke('timer_command', { message: cmd });
	}

	/**
	 * @param {SnesEvent} o
	 * @param {SnesEvent} snesEvent
	 */
	function objectiveCleared(o, snesEvent) {
		if (o === undefined) {
			console.log('objective sent to objectiveCleard is undefined');
			console.log(runObjectives.objectives);
			return false;
		}
		return (
			o.tile_id === snesEvent.tile_id &&
			o.item_id === snesEvent.item_id &&
			o.event_id === snesEvent.event_id &&
			o.location_id === snesEvent.location_id
		);
	}

	/**
	 * Resets all triggered events on rust side.
	 */
	function untriggerEvents() {
		sendTimerCommand('clear_event_log');
	}

	/**
	 * start run setting the given event as the start objective
	 *
	 * @param {SnesEvent} e - The event that triggered the run
	 */
	function startRun(e) {
		console.log('STARTING THE RUN OOO');
		untriggerEvents();
		currentRun = [e];
		currentObjective = 1;
		runStarted = true;
		runFinished = false;
	}

	/**
	 * Increment the run by one
	 *
	 * @param {SnesEvent} e - the objective that was just finished
	 */
	function progressRun(e) {
		currentRun = [...currentRun, e];
		currentObjective++;
		if (currentObjective >= runObjectives.objectives.length) {
			finishRun();
		}
	}

	function abortRun() {
		runStarted = false;
	}

	function finishRun() {
		runStarted = false;
		runFinished = true;
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

	async function startRecording() {
		runObjectives = {
			start_tile: current_tile,
			objectives: [],
			finalized: false
		};
		untriggerEvents();
		// let rust backend reset event log
		// and read in previous events again before recording
		await new Promise((r) => setTimeout(r, 100));
		isRecording = true;
	}

	function stopRecording() {
		isRecording = false;
		runObjectives.finalized = true;
	}

	onMount(async () => {
		untriggerEvents();
		unlisten_snes_events = await listen('snes_event', (event) => {
			let /** @type {SnesEvent} */ snesEvent = JSON.parse(event.payload);

			if (runStarted) {
				let objective = runObjectives.objectives[currentObjective];
				if (!objective) {
					console.log(`HEEEEY currentObject ${currentObjective} is ${objective} `);
				}
				if (objectiveCleared(objective, snesEvent)) {
					progressRun(snesEvent);
				}
			} else if (
				runObjectives.finalized &&
				runObjectives.start_tile === current_tile &&
				objectiveCleared(runObjectives.objectives[0], snesEvent)
			) {
				console.log('Why is this triggering');
				startRun(snesEvent);
			}
			if (snesEvent.tile_id) {
				current_tile = snesEvent.tile_id;
			}
			snesEvents = [...snesEvents, event.payload];
			if (isRecording) {
				runObjectives.objectives = [...runObjectives.objectives, snesEvent];
				if (snesEvent.tile_id) {
				}
			}
		});
	});
</script>

<h1>A Link to The Past Autotimer</h1>

<div style="display:grid; grid-template-columns: auto auto;">
	<div style="grid-column: 1; grid-row: 1;">
		{#if isRecording}
			<button on:click={stopRecording}>Stop Recording</button>
		{:else}
			<button on:click={startRecording}>Start Recording</button>
		{/if}
	</div>

	<div style="grid-column: 2; grid-row: 1;">
		<button disabled={!runStarted} on:click={abortRun}>Abort run</button>
	</div>

	<div style="grid-column: 1; grid-row: 2;">
		<h3>Objectives</h3>
		{#if runObjectives.start_tile !== -1}
			<p>Start: {tiles[runObjectives.start_tile].name}</p>
			<ol>
				{#each runObjectives.objectives as o}
					<li>{eventInfo(o)?.name}</li>
				{/each}
			</ol>
		{/if}
	</div>

	<div class="run-times" style="grid-column: 2; grid-row: 2;">
		{#if runStarted}
			<h3>Current Run</h3>
			<br />
			<ul>
				{#each currentRun as cleared, idx}
					<li>{fmtDelta(cleared.timestamp, currentRun[idx - 1]?.timestamp)}</li>
				{/each}
			</ul>
		{:else if runFinished}
			<h3>Finished {fmTime(runTime(currentRun) ?? 0)}</h3>
			<br />
			<ul>
				{#each currentRun as cleared, idx}
					<li>{fmtDelta(cleared.timestamp, currentRun[idx - 1]?.timestamp)}</li>
				{/each}
			</ul>
		{:else}
			<h3>No Active Run</h3>
		{/if}
	</div>

	<div style="grid-column: 3; grid-row: 2;">
		<h3>Best times</h3>
	</div>

	<div style="grid-column: 1; grid-row: 3;">
		<h3>Current Tile</h3>
		{#if current_tile !== -1}
			{#each Object.entries(tiles[current_tile]) as [k, data]}
				<p>{k}: {data}</p>
			{/each}
		{/if}
	</div>
</div>

<style>
	ul {
		list-style: none;
	}

	.run-times {
		text-align: left;
	}
</style>
