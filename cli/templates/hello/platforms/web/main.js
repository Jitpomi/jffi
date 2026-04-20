import init, { Core } from './pkg/wasm.js';

let core = null;

async function initApp() {
    await init();
    core = new Core();
    
    const greetingEl = document.getElementById('greeting');
    greetingEl.textContent = core.greeting();
    
    document.getElementById('refresh-btn').addEventListener('click', () => {
        greetingEl.textContent = core.greeting();
    });
}

initApp();
