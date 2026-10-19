import '@scutoons/tokens';
import './lib/buttons.css';
import { mount } from 'svelte';

// /phone?code=… = the handheld camera's phone page (README "Handheld camera"), everything else the studio
const page = location.pathname.startsWith('/phone') ? import('./Phone.svelte') : import('./App.svelte');
page.then(({ default: Root }) => mount(Root, { target: document.getElementById('app') }));
