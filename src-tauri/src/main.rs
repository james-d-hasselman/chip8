#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use tauri::api::dialog::FileDialogBuilder;
use tauri::WindowMenuEvent;
use tauri::{CustomMenuItem, Menu, MenuItem, Submenu};

#[derive(Clone, serde::Serialize)]
struct StopPayload {
  message: String,
}

fn main() {
  let load_rom = CustomMenuItem::new("load_rom".to_string(), "Load Rom...");
  let stop = CustomMenuItem::new("stop".to_string(), "Stop");
  let reset = CustomMenuItem::new("reset".to_string(), "Reset");
  let quit = CustomMenuItem::new("quit".to_string(), "Quit");
  let interpreter_menu = Submenu::new(
    "Interpreter",
    Menu::new()
      .add_item(load_rom)
      .add_native_item(MenuItem::Separator)
      .add_item(stop)
      .add_item(reset)
      .add_native_item(MenuItem::Separator)
      .add_item(quit),
  );
  let menu = Menu::new().add_submenu(interpreter_menu);
  tauri::Builder::default()
    .menu(menu)
    .on_menu_event(|event: WindowMenuEvent| {
      match event.menu_item_id() {
        "quit" => {
          std::process::exit(0);
        }
        "stop" => {
          event.window().emit("stop-event", StopPayload { message: "STOP!".into() }).unwrap();
          println!("FUCK");
        }
        _ => {}
      }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
