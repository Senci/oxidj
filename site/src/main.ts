import { initApp } from './ui/app';
import './styles/style.css';

const app = document.getElementById('app');
if (app === null) {
  throw new Error('Root element #app not found');
}
initApp(app);
