#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod audio;
mod chip8;
mod graphics;
mod keyboard;
mod memory;
mod registers;
mod time;

use audio::Buzzer;
use chip8::Interpreter;
use graphics::Display;
use keyboard::Keyboard;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use tauri::api::dialog::FileDialogBuilder;
use tauri::AppHandle;
use tauri::EventHandler;
use tauri::Manager;
use tauri::Window;
use tauri::WindowMenuEvent;
use tauri::{CustomMenuItem, Menu, MenuItem, State, Submenu};

#[derive(Clone, serde::Serialize)]
struct StopPayload {
    message: String,
}

#[derive(Clone, serde::Deserialize)]
struct KeyDown {
    key: String,
}

#[derive(Clone, serde::Deserialize)]
struct KeyUp {
    key: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Rom {
    path: String,
}

#[derive(Clone, serde::Serialize)]
struct Frame {
    buffer: Vec<bool>,
}

#[derive(Clone, serde::Serialize)]
struct BuzzerSpec {
    frequency: f32,
    volume: f32,
}

#[derive(Clone, serde::Serialize)]
struct JsSprite {
    x: u8,
    y: u8,
    update: Vec<Vec<bool>>,
}

struct TauriDisplay {
    buffer: [[bool; 64]; 32],
    window: tauri::Window,
}

impl TauriDisplay {
    fn draw_byte(&mut self, x: u8, y: u8, byte: u8) -> (u8, [bool; 8]) {
        let bits: Vec<bool> = {
            let byte = byte;
            (format!("{:08b}", byte))
                .chars()
                .map(|c| c.to_digit(10).expect("Memory corrupted, crashing") == 1)
                .collect()
        };

        let mut collision = 0;
        let mut updated_pixels = [false; 8];
        for (x_offset, bit) in bits.iter().enumerate() {
            let target_bit =
                &mut self.buffer[usize::from(y)][usize::from((usize::from(x) + x_offset) % 64)];
            if *bit && *target_bit {
                collision = 1;
            }
            updated_pixels[x_offset] = *target_bit ^ *bit;
            *target_bit = updated_pixels[x_offset];
        }

        return (collision, updated_pixels);
    }

    fn new(window: tauri::Window) -> Self {
        Self {
            buffer: [[false; 64]; 32],
            window,
        }
    }
}

impl Display for TauriDisplay {
    fn clear(&mut self) {
        self.buffer = [[false; 64]; 32];
        match self.window.emit("clear", ()) {
            Err(error) => {
                eprintln!("Error sending 'clear' event: {}", error);
            }
            _ => ()
        }
    }

    fn draw(&mut self, x: u8, y: u8, sprite: &graphics::Sprite) -> u8 {
        let mut collision = 0;
        let mut update = Vec::<Vec<bool>>::new();
        for (i, byte) in sprite.iter().enumerate() {
            let i = i as u8;
            let (result, updated_pixels) = self.draw_byte(x, (y + i) % 32, *byte);
            if result == 1 {
                collision = 1;
            }
            update.push(updated_pixels.to_vec());
        }

        match self.window.emit("draw-sprite", JsSprite { x, y, update }) {
            Err(error) => {
                eprintln!("Error sending 'draw-sprite' event: {}", error);
            }
            _ => ()
        }

        return collision;
    }

    fn refresh(&mut self) {
    }
}

struct JavaScriptAudio {
    window: Window,
}

impl JavaScriptAudio {
    fn new(window: Window) -> Self {
        Self { window }
    }
}

impl Buzzer for JavaScriptAudio {
    fn initialize(self, frequency: f32, volume: f32) {
        self.window
            .emit("initialize-buzzer", BuzzerSpec { frequency, volume })
            .unwrap();
    }

    fn play(&self) {
        self.window.emit("play-buzzer", ()).unwrap();
    }

    fn pause(&self) {
        self.window.emit("pause-buzzer", ()).unwrap();
    }
}

#[derive(Debug)]
struct TauriKeyboard {
    keys: Arc<Mutex<[bool; 16]>>,
    keyup_handler: Option<EventHandler>,
    keydown_handler: Option<EventHandler>,
    app_handle: AppHandle,
}

impl TauriKeyboard {
    pub fn new(app_handle: AppHandle) -> Self {
        let keys = Arc::new(Mutex::new([false; 16]));
        let mut keyboard = Self {
            keys: keys.clone(),
            keyup_handler: None,
            keydown_handler: None,
            app_handle,
        };
        let keydown_keys = keys.clone();
        keyboard.keydown_handler =
            Some(keyboard.app_handle.listen_global("keydown", move |event| {
                let mut keys = keydown_keys.lock().unwrap();
                let keydown: KeyDown = serde_json::from_str(event.payload().unwrap()).unwrap();
                match keydown.key.as_str() {
                    "Digit1" => keys[1] = true,
                    "Digit2" => keys[2] = true,
                    "Digit3" => keys[3] = true,
                    "Digit4" => keys[0xC] = true,
                    "KeyQ" => keys[4] = true,
                    "KeyW" => keys[5] = true,
                    "KeyE" => keys[6] = true,
                    "KeyR" => keys[0xD] = true,
                    "KeyA" => keys[7] = true,
                    "KeyS" => keys[8] = true,
                    "KeyD" => keys[9] = true,
                    "KeyF" => keys[0xE] = true,
                    "KeyZ" => keys[0xA] = true,
                    "KeyX" => keys[0] = true,
                    "KeyC" => keys[0xB] = true,
                    "KeyV" => keys[0xF] = true,
                    _ => (), // other keys don't matter
                }
            }));
        let keyup_keys = keys.clone();
        keyboard.keyup_handler = Some(keyboard.app_handle.listen_global("keyup", move |event| {
            let mut keys = keyup_keys.lock().unwrap();
            let keyup: KeyUp = serde_json::from_str(event.payload().unwrap()).unwrap();
            match keyup.key.as_str() {
                "Digit1" => keys[1] = false,
                "Digit2" => keys[2] = false,
                "Digit3" => keys[3] = false,
                "Digit4" => keys[0xC] = false,
                "KeyQ" => keys[4] = false,
                "KeyW" => keys[5] = false,
                "KeyE" => keys[6] = false,
                "KeyR" => keys[0xD] = false,
                "KeyA" => keys[7] = false,
                "KeyS" => keys[8] = false,
                "KeyD" => keys[9] = false,
                "KeyF" => keys[0xE] = false,
                "KeyZ" => keys[0xA] = false,
                "KeyX" => keys[0] = false,
                "KeyC" => keys[0xB] = false,
                "KeyV" => keys[0xF] = false,
                _ => (), // other keys don't matter
            }
        }));
        keyboard
    }
}

impl Keyboard for TauriKeyboard {
    fn is_key_down(&self, key: u8) -> bool {
        {
            let keys = self.keys.lock().unwrap();
            keys[key as usize]
        }
    }

    fn get_pressed_key(&self) -> Option<u8> {
        {
            let keys = self.keys.lock().unwrap();
            for (i, key) in keys.iter().enumerate() {
                if *key {
                    return Some(i as u8);
                }
            }
            None
        }
    }
}

impl Drop for TauriKeyboard {
    fn drop(&mut self) {
        self.app_handle.unlisten(self.keydown_handler.unwrap());
        self.app_handle.unlisten(self.keyup_handler.unwrap());
    }
}

#[tauri::command]
fn initialize_interpreter(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
    interpreter_state: State<InterpreterState>,
    rom: Rom,
) {
    let mut rom_file = File::open(rom.path).unwrap();
    let mut rom = vec![];
    rom_file.read_to_end(&mut rom).unwrap();
    let display = TauriDisplay::new(window.clone());
    let keyboard = TauriKeyboard::new(app_handle.clone());
    let buzzer = JavaScriptAudio::new(window.clone());
    let mut interpreter = Interpreter::new(
        Box::new(display),
        Box::new(buzzer),
        Box::new(keyboard),
        &rom,
    );
    interpreter_state.is_running.store(true, Ordering::Relaxed);
    let thread_is_running = interpreter_state.is_running.clone();
    *interpreter_state.interpreter_thread.lock().unwrap() = Some(std::thread::spawn(move || {
        while thread_is_running.load(Ordering::Relaxed) {
            interpreter.run_iteration();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }));
    println!("initialized");
}

#[derive(Default)]
struct InterpreterState {
    interpreter_thread: std::sync::Mutex<Option<JoinHandle<()>>>,
    is_running: Arc<AtomicBool>,
}

impl Drop for InterpreterState {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        self.interpreter_thread
            .lock()
            .unwrap()
            .take()
            .map(JoinHandle::join);
    }
}

fn main() {
    let load_rom = CustomMenuItem::new("load_rom".to_string(), "Load Rom...");
    let stop = CustomMenuItem::new("stop".to_string(), "Stop");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let interpreter_menu = Submenu::new(
        "Interpreter",
        Menu::new()
            .add_item(load_rom)
            .add_native_item(MenuItem::Separator)
            .add_item(stop)
            .add_native_item(MenuItem::Separator)
            .add_item(quit),
    );
    let menu = Menu::new().add_submenu(interpreter_menu);
    tauri::Builder::default()
        .manage(InterpreterState {
            interpreter_thread: Mutex::new(None),
            is_running: Arc::new(AtomicBool::new(false)),
        })
        .menu(menu)
        .on_menu_event(|event: WindowMenuEvent| match event.menu_item_id() {
            "quit" => {
                std::process::exit(0);
            }
            "stop" => {
                let window = event.window();
                let interpreter_state = window.state::<InterpreterState>();
                interpreter_state.is_running.store(false, Ordering::Relaxed);
                event.window().emit("stop", ()).unwrap();
            }
            "load_rom" => {
                let window = event.window();
                let interpreter_state = window.state::<InterpreterState>();
                interpreter_state.is_running.store(false, Ordering::Relaxed);
                FileDialogBuilder::new().pick_file(move |path| {
                    match path {
                      Some(path) => {
                          event
                        .window()
                        .emit(
                            "rom-loaded",
                            Rom {
                                path: path.into_os_string().into_string().unwrap(),
                            },
                        )
                        .unwrap();
                      }
                      _ => ()
                    }
                });
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![initialize_interpreter,])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
