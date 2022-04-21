import './style.css'
import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

let application = document.querySelector('#application');
let game_container = document.querySelector('#game-container');
let display = document.querySelector('#display');

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
  update.forEach((byte, y_offset) => {
    byte.forEach((bit, x_offset) => {
      let x = ((update_x + x_offset) % 64);
      let y = ((update_y + y_offset) % 32);
      if (bit) {
        display.children[y].children[x].classList.add("on");
      } else {
        display.children[y].children[x].classList.remove("on");
      }
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

let clearDisplay = () => {
  for(let y = 0; y < 32; y++) {
    for(let x = 0; x < 64; x++) {
      display.children[y].children[x].classList.remove("on");
    }
  }
}

window.onload = () => {
  for(var y = 0; y < 32; y++)
  {
    let row = document.createElement('div');
    row.classList.add("row")
    for(var x = 0; x < 64; x++) {
      let pixel = document.createElement('div');
      pixel.classList.add("pixel");
      row.appendChild(pixel);
    }
    display.appendChild(row);
  }
  clearDisplay();
}