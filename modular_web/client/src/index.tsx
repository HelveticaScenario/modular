import { h, render } from 'deps/preact';
import { App } from './app';

const root = document.getElementById('root');
if (!root) {
    throw new Error('root not found');
}

render(<App start={0} />, root);
