<script>
    import { onMount } from 'svelte'
    import { listen } from '@tauri-apps/api/event'
    import { invoke } from '@tauri-apps/api/tauri'

    /** 
     * @typedef {Object} Message
     * @property {number} timestamp
     * @property {string} message
     */

    let unlisten_rs2js
    let unlisten_snes_events

    let output = ''
    /** @type {Message[]} outputs */
    let outputs = []
    /** @type {Message[]} inputs */
    let inputs = []
    let name = ''
    let greetMsg = ''


    async function sendOutput() {
        console.log("THIS IS SVELTE js2rs: " + output)
        outputs = [...outputs, { timestamp: Date.now(), message: output }]
        invoke('js2rs', { message: output })
    }

    onMount(async () => {
        unlisten_rs2js = await listen('rs2js', (event) => {
            console.log("THIS IS SVELTE rs2js: " + event.payload)
            let input = event.payload
            inputs = [...inputs, {timestamp: Date.now(), message: input}]
        })
        unlisten_snes_events = await listen('snes_event', (event) => {
            console.log(event.payload)
        });
    })

    async function greet() {
        greetMsg = await invoke('greet', { name })
    }
</script>

<h1>Welcome to SvelteKit</h1>
<p>Visit <a href="https://kit.svelte.dev">kit.svelte.dev</a> to read the documentation</p>

<div style="display:grid; grid-template-columns: auto auto;">
    <input id="greet-input" placeholder="Enter a name.." bind:value="{name}" />
    <button on:click="{greet}">Greet</button>
    <p>{greetMsg}</p>

    <div style="grid-column: span 2; grid-row: 1;">
        <label for="messageInput" style="display: block;">
        <input id="messageInput" bind:value={output}>
        <br>
        <button on:click={sendOutput}>Send to Rust</button>
    </div>
    <div style="grid-column: 1; grid-row: 2;">
        <h3>Output</h3>
        <ol>
            {#each outputs as output}
                <li>{output.timestamp}: {output.message}</li>
            {/each}
        </ol>
    </div>
    <div style="grid-column: 2; grid-row: 2;">
        <h3>Input</h3>
        <ol>
            {#each inputs as inp}
                <li>{inp.timestamp}: {inp.message}</li>
            {/each}
        </ol>
    </div>
</div>

