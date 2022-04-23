import './style.css'
import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

let game_container = document.querySelector('#game-container');
let canvas = document.querySelector('#display');
let display = canvas.getContext('2d', { alpha: false });
let canvas_buffer = document.createElement('canvas');
let display_buffer = canvas_buffer.getContext('2d', { alpha: false });
var pixel_size = 0;

const AudioContext = window.AudioContext || window.webkitAudioContext;
var audio_context = null;
var oscillator = null;
var gain_node = null;

window.addEventListener('keydown', e => emit('keydown', { key: `${e.code}` }));
window.addEventListener('keyup', e => emit('keyup', { key: `${e.code}` }));
window.addEventListener('contextmenu', e => {
  e.preventDefault();
  return false;
});

listen('rom-loaded', event => {
  clearDisplay();
  invoke('initialize_interpreter', { rom: event.payload });
})
listen('stop', () => {
  clearDisplay();
})

listen('play-buzzer', () => {
  oscillator.start();
})

listen('pauze-buzzer', () => {
  oscillator.stop();
})

listen('draw-sprite', event => {
  let update_x = event.payload.x;
  let update_y = event.payload.y;
  let update = event.payload.update;
  update.forEach((byte, y_offset) => {
    byte.forEach((bit, x_offset) => {
      let x = ((update_x + x_offset) % 64) * pixel_size;
      let y = ((update_y + y_offset) % 32) * pixel_size;
      if (bit) {
        display_buffer.fillStyle = "#FFFFFF";
      } else {
        display_buffer.fillStyle = "#000000";
      }
      display_buffer.fillRect(x, y, pixel_size, pixel_size)
    })
  })
  window.requestAnimationFrame(() => {
    display.drawImage(canvas_buffer, 0, 0);
  })
});

listen('clear', () => {
  clearDisplay();
});

let resizeDisplay = () => {
  var height = Math.floor(game_container.offsetHeight);
  var width = Math.floor(game_container.offsetWidth);
  let aspect_ratio = width / height;
  if (aspect_ratio > 2.0) {
    height = Math.floor(height / 32) * 32;
    width = height * 2;
    canvas.style.width = width + 'px';
    canvas.width = width;
    canvas.style.height = height + 'px';
    canvas.height = height;
  } else {
    width = Math.floor(width / 64) * 64;
    height = width / 2;
    canvas.style.width = width + 'px';
    canvas.width = width;
    canvas.style.height = height + 'px';
    canvas.height = height;
  }
  pixel_size = Math.floor(canvas.height / 32);
  canvas_buffer.width = canvas.width;
  canvas_buffer.height = canvas.height;
};

let clearDisplay = () => {
  display_buffer.fillStyle = "#000000";
  display_buffer.fillRect(0.0, 0.0, canvas_buffer.width, canvas_buffer.height);
  window.requestAnimationFrame(() => {
    display.drawImage(canvas_buffer, 0, 0);
  });
}

window.addEventListener('resize', resizeDisplay);
window.onload = () => {
  resizeDisplay();
  clearDisplay();
  audio_context = new AudioContext();
  oscillator = audio_context.createOscillator();
  oscillator.type = 'square';
  oscillator.frequency.setValueAtTime(600, audio_context.currentTime);
  gain_node = audio_context.createGain();
  gain_node.gain.value = 0.2;
  oscillator.connect(gain_node);
  gain_node.connect(audio_context.destination);
}