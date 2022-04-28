/* chip8 - A cross platform CHIP-8 interpreter.
 * Copyright (C) 2022  James D. Hasselman
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

import './style.css'
import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

let game_container = document.querySelector('#game-container');
let canvas = document.querySelector('#display');
let display = canvas.getContext('2d', { alpha: false });
let canvas_buffer = document.createElement('canvas');
let display_buffer = canvas_buffer.getContext('2d', { alpha: false });
var pixel_size = 0;

//const AudioContext = window.AudioContext || window.webkitAudioContext;
var audio_context = null;
var oscillator = null;
var gain_node = null;
var is_audio_started = false;

window.addEventListener('keydown', e => emit('keydown', { key: `${e.code}` }));
window.addEventListener('keyup', e => emit('keyup', { key: `${e.code}` }));
window.addEventListener('contextmenu', e => {
  e.preventDefault();
  return false;
});

listen('rom-loaded', event => {
  if (!is_audio_started) {
    oscillator.start();
    is_audio_started = true;
  }
  clearDisplay();
  invoke('initialize_interpreter', { rom: event.payload });
})
listen('stop', () => {
  clearDisplay();
})

listen('play-buzzer', () => {
  gain_node.gain.value = 0.005;
})

listen('pause-buzzer', () => {
  gain_node.gain.value = 0.0;
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
  gain_node.gain.value = 0.0;
  oscillator.connect(gain_node);
  gain_node.connect(audio_context.destination);
}
