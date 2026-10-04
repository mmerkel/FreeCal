import 'temporal-polyfill/global';
import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { tauriCore } from './core/tauriCore';

mount(App, {
  target: document.getElementById('app')!,
  props: { core: tauriCore },
});
