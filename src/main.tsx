import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { FocusStyleManager } from '@blueprintjs/core';
import App from './App';
import './styles/globals.css';

FocusStyleManager.onlyShowFocusOnTabs();

createRoot(document.getElementById('root')!).render(
  <BrowserRouter>
    <App />
  </BrowserRouter>
);
