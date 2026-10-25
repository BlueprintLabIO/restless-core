import { mount } from 'svelte';
import MathCompatibility from './MathCompatibility.svelte';

const target = document.querySelector('main');
if (!target) throw new Error('Math compatibility fixture needs its mount point');
mount(MathCompatibility, { target });
