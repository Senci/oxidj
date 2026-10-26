import { initApp } from './ui/app';
const app = document.getElementById('main');
if (app === null) throw new Error('Root element #main not found');
initApp(app);
