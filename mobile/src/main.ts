import '$shared/design/theme.css';
import { mount } from 'svelte';
import { applyTokens } from '$shared/design/theme';
import App from './App.svelte';
import './mobile.css';

applyTokens();
mount(App, { target: document.getElementById('app')! });
