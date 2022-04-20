import './style.css'
import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

let application = document.querySelector('#application');
let game_container = document.querySelector('#game-container');
let canvas = document.querySelector('#display');
let display = canvas.getContext('2d');

window.addEventListener('keydown', e => emit('keydown', { key: `${e.code}` }));
window.addEventListener('keyup', e => emit('keyup', { key: `${e.code}` }));

const unlisten_rom_loaded = listen('rom-loaded', event => {
  invoke('initialize_interpreter', { rom: event.payload });
})

const unlisten_stop = listen('stop', event => {
  clearDisplay();
})

const unlisten_start = listen('start', event => {
  window.requestAnimationFrame(interpreter_loop);
})

const unlisten_play_buzzer = listen('play-buzzer', event => {

})

const unlisten_pause_buzzer = listen('pauze-buzzer', event => {

})

const unlisten_animation_frame = listen('animation-frame', event => {
  clearDisplay();
  let buffer = event.payload.buffer;
  display.fillStyle = "#FFFFFF";
  let pixel_size = Math.floor(canvas.height / 32);
  buffer.forEach((pixel, index) => {
    if (pixel) {
      let column = index % 64;
      let row = Math.floor(index / 64);
      let x = column * (pixel_size);
      let y = row * (pixel_size);
      display.fillRect(x, y, pixel_size, pixel_size);
    }
  });
  display.stroke();
})

const unlisten_draw_sprite = listen('draw-sprite', event => {
  let update_x = event.payload.x;
  let update_y = event.payload.y;
  let update = event.payload.update;
  let pixel_size = Math.floor(canvas.height / 32);
  update.forEach((byte, y_offset) => {
    byte.forEach((bit, x_offset) => {
      let x = ((update_x + x_offset) % 64) * pixel_size;
      let y = ((update_y + y_offset) % 32) * pixel_size;
      if (bit) {
        display.fillStyle = "#FFFFFF";
      } else {
        display.fillStyle = "#000000";
      }
      display.fillRect(x, y, pixel_size, pixel_size)
    })
  })
})

let is_running = true;
let interpreter_loop = () => {
  if (is_running) {
    window.requestAnimationFrame(interpreter_loop);
    is_running = invoke('run_iteration');
  }
}

let resizeDisplay = () => {
  let height = game_container.offsetHeight;
  let width = game_container.offsetWidth;
  let aspect_ratio = width / height;
  if (aspect_ratio > 2.0) {
    let width = height * 2.0;
    canvas.style.width = width;
    canvas.width = width;
    canvas.style.height = height;
    canvas.height = height;
  } else {
    let height = width / 2.0;
    canvas.style.width = width;
    canvas.width = width;
    canvas.style.height = height;
    canvas.height = height;
  }
};

let clearDisplay = () => {
  display.fillStyle = "#000000";
  display.fillRect(0.0, 0.0, canvas.width, canvas.height);
}

window.addEventListener('resize', resizeDisplay);
window.onload = () => {
  resizeDisplay();
  clearDisplay();
}