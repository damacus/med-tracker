import { copyFile } from 'node:fs/promises';

await copyFile('node_modules/html5-qrcode/html5-qrcode.min.js', 'assets/static/js/html5-qrcode.min.js');
